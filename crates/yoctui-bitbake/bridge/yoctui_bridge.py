#!/usr/bin/env python3
"""NDJSON BitBake bridge. Diagnostics are deliberately written only to stderr."""

import importlib
import json
import math
import os
import re
import selectors
import subprocess
import sys
import time

VERSION = 1
MAX_LINE_BYTES = 1024 * 1024
MAX_DEPENDENCY_NODES = 1500
MAX_DEPENDENCY_EDGES = 3000
MAX_NATIVE_EVENTS_PER_POLL = 64
sequence = 0
protocol_output = sys.stdout


def isolate_protocol_output():
    """Keep BitBake and child-process stdout away from the NDJSON channel."""
    global protocol_output
    protocol_fd = os.dup(sys.stdout.fileno())
    protocol_output = os.fdopen(
        protocol_fd,
        "w",
        buffering=1,
        encoding=sys.stdout.encoding or "utf-8",
        errors="replace",
    )
    os.dup2(sys.stderr.fileno(), sys.stdout.fileno())


def emit(message, correlation_id=None):
    global sequence
    sequence += 1
    value = {"protocol_version": VERSION, "sequence": sequence, "message": message}
    if correlation_id is not None:
        value["correlation_id"] = correlation_id
    protocol_output.write(
        json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n"
    )
    protocol_output.flush()


def error(code, message, correlation_id=None):
    emit({"type": "command_failed", "code": code, "message": message}, correlation_id)


def bitbake_version():
    override = os.environ.get("YOCTUI_BITBAKE_VERSION")
    if override:
        return override
    try:
        import bb  # type: ignore[import-not-found]

        return getattr(bb, "__version__", None)
    except ImportError:
        return None


class CompatibilityError(Exception):
    pass


class ServerUnavailable(Exception):
    pass


class TinfoilConnection:
    """Thin production adapter around BitBake's supported Tinfoil API."""

    EVENT_MASK = [
        "bb.event.BuildStarted",
        "bb.event.BuildCompleted",
        "bb.event.ParseStarted",
        "bb.event.ParseProgress",
        "bb.event.ParseCompleted",
        "bb.event.ProcessStarted",
        "bb.event.ProcessProgress",
        "bb.event.ProcessFinished",
        "bb.command.CommandCompleted",
        "bb.command.CommandFailed",
        "bb.command.CommandExit",
        "bb.build.TaskStarted",
        "bb.build.TaskSucceeded",
        "bb.build.TaskFailed",
        "bb.build.TaskFailedSilent",
        "bb.build.TaskProgress",
        "bb.runqueue.runQueueTaskStarted",
        "bb.runqueue.sceneQueueTaskStarted",
        "logging.LogRecord",
    ]

    native_event_stream = True

    def __init__(self, module):
        self.module = module
        self.tinfoil_module = importlib.import_module("bb.tinfoil")
        self.tinfoil = None
        self.recipes_parsed = False
        self.active = False
        self._prepare()

    def _prepare(self):
        self.tinfoil = self.tinfoil_module.Tinfoil(
            output=sys.stderr, tracking=True, setup_logging=True
        )
        self.tinfoil.prepare(config_only=True, quiet=2)
        self.recipes_parsed = False
        self.active = False
        self.recipe_files = {}
        self.force_active = False

    def _ensure_recipes(self):
        if not self.recipes_parsed:
            self.tinfoil.parse_recipes()
            self.recipes_parsed = True

    def _reset_for_build(self):
        # Metadata queries parse recipes synchronously. A fresh config-only
        # connection lets the subsequent build expose its real parse events.
        if self.recipes_parsed:
            self.tinfoil.shutdown()
            self._prepare()

    def _variable_operations(self, datastore, name):
        try:
            history = datastore.varhistory.variable(name) or []
        except Exception:
            return []
        operations = []
        for item in history:
            if not isinstance(item, dict) or "flag" in item:
                continue
            path = item.get("file")
            line = item.get("line")
            detail = item.get("detail")
            operations.append(
                {
                    "operation": str(item.get("op") or "set"),
                    "file": path if isinstance(path, str) else None,
                    "line": line
                    if isinstance(line, int) and not isinstance(line, bool)
                    else None,
                    "value": None if detail is None else str(detail),
                }
            )
        return operations

    def _variable_provenance(self, datastore, name):
        sources = []
        for operation in self._variable_operations(datastore, name):
            path = operation["file"]
            line = operation["line"]
            if path is not None:
                sources.append(f"{path}:{line}" if line is not None else path)
        return sources[-1] if sources else None

    def _layers(self):
        priorities = self.tinfoil.run_command("getLayerPriorities") or []
        configured = (self.tinfoil.config_data.getVar("BBLAYERS") or "").split()
        layers = []
        for collection, _pattern, regex, priority in priorities:
            path = next(
                (
                    candidate
                    for candidate in configured
                    if re.match(regex, candidate.rstrip("/") + "/")
                ),
                None,
            )
            if path is None:
                path = regex.removeprefix("^").removesuffix("/")
            layers.append(
                {
                    "name": str(collection),
                    "path": path,
                    "priority": int(priority),
                }
            )
        return layers

    def inspect_workspace(self):
        keys = (
            "MACHINE",
            "DISTRO",
            "BBLAYERS",
            "DL_DIR",
            "SSTATE_DIR",
            "TMPDIR",
            "DEPLOY_DIR_IMAGE",
            "PKGDATA_DIR",
            "IMAGE_MANIFEST",
            "IMAGE_ROOTFS",
            "WKS_FILE",
            "WKS_FILES",
            "WKS_SEARCH_PATH",
            "WKS_FILES_DIR",
            "PACKAGE_CLASSES",
            "BB_NUMBER_THREADS",
            "PARALLEL_MAKE",
        )
        variables = {}
        provenance = {}
        for key in keys:
            value = self.tinfoil.config_data.getVar(key)
            if value is not None:
                variables[key] = str(value)
            source = self._variable_provenance(self.tinfoil.config_data, key)
            if source is not None:
                provenance[key] = source
        return {
            "build_dir": self.tinfoil.config_data.getVar("TOPDIR"),
            "source_dir": self.tinfoil.config_data.getVar("COREBASE"),
            "variables": variables,
            "variable_provenance": provenance,
            "variable_provenance_chain": {},
            "bitbake_version": getattr(self.module, "__version__", None),
            "release": self.tinfoil.config_data.getVar("DISTRO_VERSION"),
            "layers": self._layers(),
            "recipes": [],
        }

    def list_layers(self):
        return self._layers()

    def _layer_for_path(self, path, layers):
        matches = [
            layer
            for layer in layers
            if path == layer["path"] or path.startswith(layer["path"].rstrip("/") + "/")
        ]
        if not matches:
            return None
        return max(matches, key=lambda layer: len(layer["path"]))["name"]

    def list_recipes(self, filter_value):
        self._ensure_recipes()
        recipes = self.tinfoil.run_command("getRecipes", "") or []
        versions = self.tinfoil.run_command("getRecipeVersions", "") or {}
        try:
            providers = self.tinfoil.run_command("findProviders", "") or ()
            preferred = (
                providers[1]
                if isinstance(providers, (list, tuple))
                and len(providers) > 1
                and isinstance(providers[1], dict)
                else {}
            )
        except Exception:
            preferred = {}
        try:
            all_appends = self.tinfoil.run_command("getAllAppends", "") or []
        except Exception:
            all_appends = []
        layers = self._layers()
        result = []
        for name, paths in recipes:
            if filter_value is not None and filter_value.lower() not in name.lower():
                continue
            recipe_paths = sorted(path for path in paths if isinstance(path, str))
            preferred_data = preferred.get(name)
            path = (
                preferred_data[1]
                if isinstance(preferred_data, (list, tuple))
                and len(preferred_data) > 1
                and isinstance(preferred_data[1], str)
                else recipe_paths[0]
                if recipe_paths
                else None
            )
            version_data = versions.get(path) if path is not None else None
            version = (
                str(version_data[1])
                if isinstance(version_data, (list, tuple)) and len(version_data) > 1
                else None
            )
            append_count = None
            if path is not None and isinstance(all_appends, (list, tuple)):
                basename = os.path.basename(path)
                append_count = sum(
                    1
                    for item in all_appends
                    if isinstance(item, (list, tuple))
                    and len(item) > 1
                    and isinstance(item[0], str)
                    and (
                        item[0] == basename
                        or (
                            "%" in item[0]
                            and item[0].startswith(basename[: item[0].index("%")])
                        )
                    )
                )
            if path is not None:
                self.recipe_files[str(name)] = path
            result.append(
                {
                    "name": str(name),
                    "version": version,
                    "layer": self._layer_for_path(path, layers)
                    if path is not None
                    else None,
                    "preferred_version": None,
                    "file": path,
                    "append_count": append_count,
                }
            )
        return result

    def get_variable(self, name, recipe):
        datastore = self.tinfoil.config_data
        if recipe is not None:
            self._ensure_recipes()
            datastore = self.tinfoil.parse_recipe(recipe)
        value = datastore.getVar(name)
        unexpanded_value = datastore.getVar(name, False)
        operations = self._variable_operations(datastore, name)
        active_overrides = [
            override
            for override in str(datastore.getVar("OVERRIDES") or "").split(":")
            if override
        ]
        return {
            "recipe": recipe,
            "value": None if value is None else str(value),
            "unexpanded_value": None
            if unexpanded_value is None
            else str(unexpanded_value),
            "provenance": next(
                (
                    f"{operation['file']}:{operation['line']}"
                    if operation["line"] is not None
                    else operation["file"]
                    for operation in reversed(operations)
                    if operation["file"] is not None
                ),
                None,
            ),
            "operations": operations,
            "active_overrides": active_overrides,
        }

    def get_rootfs_sources(self, recipe):
        """Return only BitBake-expanded paths for one exact image recipe."""
        self._ensure_recipes()
        datastore = self.tinfoil.parse_recipe(recipe)
        return {
            name.lower(): (
                None if datastore.getVar(name) is None else str(datastore.getVar(name))
            )
            for name in ("IMAGE_MANIFEST", "PKGDATA_DIR", "IMAGE_ROOTFS")
        }

    def get_dependencies(self, recipe):
        self._ensure_recipes()
        datastore = self.tinfoil.parse_recipe(recipe)
        build = (datastore.getVar("DEPENDS") or "").split()
        runtime = (
            datastore.getVar(f"RDEPENDS:{recipe}") or datastore.getVar("RDEPENDS") or ""
        ).split()
        return {"build": build, "runtime": runtime}

    def get_dependency_graph(self, recipe):
        if self.active:
            raise RuntimeError(
                "dependency graphs are unavailable during an active build"
            )
        self._ensure_recipes()
        event_mask = [
            "bb.event.DepTreeGenerated",
            "bb.command.CommandCompleted",
            "bb.command.CommandFailed",
            "bb.command.CommandExit",
            "logging.LogRecord",
        ]
        self.tinfoil.set_event_mask(event_mask)
        default_task = self.tinfoil.config_data.getVar("BB_DEFAULT_TASK") or "build"
        graph_data = None
        try:
            self.tinfoil.run_command(
                "generateDepTreeEvent",
                [recipe],
                default_task,
                handle_events=False,
            )
            deadline = time.monotonic() + 120
            while time.monotonic() < deadline:
                event = self.tinfoil.wait_event(0.25)
                if event is None:
                    continue
                kind = type(event).__name__
                if kind == "DepTreeGenerated":
                    graph_data = getattr(event, "_depgraph", None)
                elif kind in ("CommandFailed", "CommandExit"):
                    raise RuntimeError(f"BitBake {kind} while generating dependencies")
                elif kind == "CommandCompleted":
                    if graph_data is None:
                        raise RuntimeError(
                            "BitBake completed dependency generation without a graph"
                        )
                    return dependency_graph_from_deptree(recipe, graph_data)
            raise RuntimeError(
                "BitBake dependency generation timed out after 120 seconds"
            )
        finally:
            self.tinfoil.set_event_mask(self.EVENT_MASK)

    def _preferred_recipe_file(self, recipe):
        if recipe in self.recipe_files:
            return self.recipe_files[recipe]
        providers = self.tinfoil.run_command("findProviders", "") or ()
        preferred = (
            providers[1]
            if isinstance(providers, (list, tuple))
            and len(providers) > 1
            and isinstance(providers[1], dict)
            else {}
        )
        preferred_data = preferred.get(recipe)
        if (
            isinstance(preferred_data, (list, tuple))
            and len(preferred_data) > 1
            and isinstance(preferred_data[1], str)
        ):
            return preferred_data[1]
        recipes = self.tinfoil.run_command("getRecipes", "") or []
        for name, paths in recipes:
            if name == recipe:
                candidates = sorted(path for path in paths if isinstance(path, str))
                if candidates:
                    return candidates[0]
        raise RuntimeError(f"no provider file is available for {recipe}")

    def get_recipe_sources(self, recipe):
        self._ensure_recipes()
        recipe_file = self._preferred_recipe_file(recipe)
        appends = self.tinfoil.get_file_appends(recipe_file) or []
        return [recipe_file, *appends]

    def get_recipe_metadata(self, recipe):
        self._ensure_recipes()
        recipe_file = self._preferred_recipe_file(recipe)
        appends = list(self.tinfoil.get_file_appends(recipe_file) or [])
        datastore = self.tinfoil.parse_recipe_file(recipe_file)
        tasks = datastore.getVar("__BBTASKS") or []
        packages = (datastore.getVar("PACKAGES") or "").split()
        source_uri = (datastore.getVar("SRC_URI") or "").split()
        patch_uris = [
            value
            for value in source_uri
            if value.split(";", 1)[0].endswith((".patch", ".diff"))
        ]
        patches = []
        try:
            fetch = importlib.import_module("bb.fetch2").Fetch(source_uri, datastore)
        except Exception:
            fetch = None
        for value in patch_uris:
            uri = value.split(";", 1)[0]
            if uri.startswith("file://") and fetch is not None:
                try:
                    patches.append(fetch.localpath(value))
                    continue
                except Exception:
                    pass
            patches.append(value)
        return {
            "recipe": recipe,
            "workspace_status": None,
            "build_status": None,
            "tasks": sorted(str(task) for task in tasks),
            "sources": [recipe_file, *appends],
            "patches": patches,
            "packages": packages,
            "history": None,
        }

    def start_build(self, targets, task, force=False):
        if self.active:
            raise RuntimeError("a BitBake build is already active")
        self._reset_for_build()
        self.tinfoil.set_event_mask(self.EVENT_MASK)
        selected_task = task or self.tinfoil.config_data.getVar("BB_DEFAULT_TASK")
        self.active = True
        try:
            if force:
                # BitBake's setConfig command coerces values to strings and
                # later tests this configuration field by truthiness.
                self.tinfoil.run_command("setConfig", "force", "1")
                self.force_active = True
            self.tinfoil.run_command(
                "buildTargets", targets, selected_task, handle_events=False
            )
        except Exception:
            self.active = False
            if self.force_active:
                self.tinfoil.run_command("setConfig", "force", "")
                self.force_active = False
            raise

    def cancel_build(self):
        if not self.active:
            raise RuntimeError("no BitBake build is active")
        # Cancellation is an explicit request to stop this runqueue.  Some
        # maintained BitBake generations let stateShutdown drain running work
        # for longer than Yoctui's bounded cancellation contract.  Use the
        # supported cooker force-shutdown command here; this remains a typed
        # Tinfoil/server operation and does not signal or kill an arbitrary
        # host process.
        self.tinfoil.run_command("stateForceShutdown", handle_events=False)

    def drain_events(self):
        events = []
        first = True
        while len(events) < MAX_NATIVE_EVENTS_PER_POLL:
            # A short first wait pumps the event socket after the runqueue
            # becomes idle. Pure zero-timeout polling can leave the final
            # BuildCompleted record unread until another server command.
            event = self.tinfoil.wait_event(0.01 if first else 0)
            first = False
            if event is None:
                break
            events.append(event)
            if type(event).__name__ == "BuildCompleted":
                self.active = False
        if not self.active and self.force_active:
            self.tinfoil.run_command("setConfig", "force", "")
            self.force_active = False
        return events

    def shutdown(self):
        if self.tinfoil is not None:
            if self.force_active:
                self.tinfoil.run_command("setConfig", "force", "")
                self.force_active = False
            self.tinfoil.shutdown()
            self.tinfoil = None
        self.active = False

    def terminate_server(self):
        """Terminate through the process-server interface used by bitbake -m."""
        if self.tinfoil is None or self.tinfoil.server_connection is None:
            raise RuntimeError("BitBake server connection is unavailable")
        connection = self.tinfoil.server_connection
        connection.connection.terminateServer()
        connection.terminate()
        try:
            self.tinfoil_module._server_connections.remove(connection)
        except (AttributeError, ValueError):
            pass
        self.tinfoil.server_connection = None
        self.tinfoil = None
        self.active = False


class BitBakeAdapter:
    def __init__(self, version, module=None):
        self.version = version
        self.module = module
        self.connection = None
        self.build_correlation_id = None
        self.build_active = False
        self.task_identities_by_pid = {}
        self.native_event_iterator = None
        self.compatibility_generation = None
        self.negotiated_capabilities = set()

    def negotiate(self, requested):
        """Directly retain only API behavior exposed by this connection."""
        connection = self.server()
        operations = {
            "bitbake.workspace_inspection": ("inspect_workspace",),
            "bitbake.recipe_inventory": ("list_recipes",),
            "bitbake.recipe_dependencies": ("get_dependencies",),
            "bitbake.recipe_sources": ("get_recipe_sources",),
            "bitbake.recipe_metadata": ("get_recipe_metadata",),
            "bitbake.layer_inventory": ("list_layers",),
            "bitbake.layer_relationships": ("get_layer_relationships",),
            "bitbake.build": ("start_build",),
            "bitbake.cancellation": ("cancel_build",),
            "bitbake.task_list": ("get_recipe_metadata",),
            "bitbake.dependency_graph": ("get_dependency_graph",),
            "bitbake.getvar": ("get_variable",),
            "bitbake.variable_history": ("get_variable",),
            "bitbake.server_socket": ("terminate_server",),
            "bitbake.native_events": ("drain_events",),
        }
        negotiated = set()
        for capability in requested:
            capability_id = capability["id"]
            methods = operations.get(capability_id)
            if methods and all(
                callable(getattr(connection, name, None)) for name in methods
            ):
                if capability_id != "bitbake.native_events" or bool(
                    getattr(connection, "native_event_stream", False)
                ):
                    negotiated.add(capability_id)
        self.negotiated_capabilities = negotiated
        return sorted(negotiated)

    def workspace(self):
        operation = self.optional_server_operation("inspect_workspace")
        if operation is None:
            return workspace_data(self.version)
        try:
            response = operation()
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect the BitBake workspace from the server: {exc}"
            )
        return {"type": "workspace", "data": typed_workspace(response)}

    def server(self):
        if self.connection is not None:
            return self.connection
        if self.module is not None and getattr(self.module, "__path__", None):
            try:
                self.connection = TinfoilConnection(self.module)
                return self.connection
            except (ImportError, AttributeError):
                pass
            except Exception as exc:
                raise ServerUnavailable(f"could not initialize BitBake Tinfoil: {exc}")
        server = getattr(self.module, "server", None) if self.module else None
        connector = getattr(server, "connect", None)
        if not callable(connector):
            raise ServerUnavailable(
                "no supported BitBake server connector is available; start BitBake and expose bb.server.connect"
            )
        try:
            self.connection = connector()
            return self.connection
        except Exception as exc:
            raise ServerUnavailable(f"could not connect to the BitBake server: {exc}")

    def start_build(self, targets, task, force=False):
        connection = self.server()
        self.task_identities_by_pid.clear()
        self.native_event_iterator = None
        operation = getattr(connection, "start_build", None)
        if not callable(operation):
            raise ServerUnavailable(
                "connected BitBake server does not provide start_build"
            )
        try:
            try:
                operation(targets, task, force)
            except TypeError:
                if force:
                    raise ServerUnavailable(
                        "connected BitBake server does not support forced task execution"
                    )
                operation(targets, task)
        except Exception as exc:
            raise ServerUnavailable(f"could not start the BitBake build: {exc}")
        self.build_active = True
        return bool(getattr(connection, "native_event_stream", False))

    def cancel_build(self):
        connection = self.server()
        operation = getattr(connection, "cancel_build", None)
        if not callable(operation):
            raise ServerUnavailable(
                "connected BitBake server does not provide cancel_build"
            )
        try:
            operation()
        except Exception as exc:
            raise ServerUnavailable(f"could not cancel the BitBake build: {exc}")
        return bool(getattr(connection, "native_event_stream", False))

    def shutdown(self):
        if self.connection is None:
            return
        operation = getattr(self.connection, "shutdown", None)
        if callable(operation):
            operation()
        self.connection = None
        self.build_active = False
        self.task_identities_by_pid.clear()
        self.native_event_iterator = None

    def optional_server_operation(self, name):
        if self.module is None:
            return None
        try:
            connection = self.server()
        except ServerUnavailable:
            return None
        operation = getattr(connection, name, None)
        return operation if callable(operation) else None

    def variable(self, name, recipe):
        """Query a server-provided effective value without interpreting metadata."""
        operation = self.optional_server_operation("get_variable")
        if operation is None:
            return None
        try:
            response = operation(name, recipe)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not query {name} from the BitBake server: {exc}"
            )
        if response is None or isinstance(response, str):
            return {"value": response, "provenance": None}
        if isinstance(response, dict):
            value = response.get("value")
            provenance = response.get("provenance")
            unexpanded_value = response.get("unexpanded_value")
            operations = response.get("operations", [])
            active_overrides = response.get("active_overrides", [])
            scope = response.get("recipe", recipe)
            if (
                (value is None or isinstance(value, str))
                and (provenance is None or isinstance(provenance, str))
                and (unexpanded_value is None or isinstance(unexpanded_value, str))
            ):
                if scope is not None and not isinstance(scope, str):
                    raise ServerUnavailable(
                        f"BitBake server returned an invalid variable scope for {name}"
                    )
                if not isinstance(active_overrides, list) or not all(
                    isinstance(item, str) for item in active_overrides
                ):
                    raise ServerUnavailable(
                        f"BitBake server returned invalid active overrides for {name}"
                    )
                if not isinstance(operations, list) or not all(
                    isinstance(item, dict)
                    and isinstance(item.get("operation"), str)
                    and (item.get("file") is None or isinstance(item.get("file"), str))
                    and (
                        item.get("line") is None
                        or (
                            isinstance(item.get("line"), int)
                            and not isinstance(item.get("line"), bool)
                        )
                    )
                    and (
                        item.get("value") is None or isinstance(item.get("value"), str)
                    )
                    for item in operations
                ):
                    raise ServerUnavailable(
                        f"BitBake server returned invalid variable operations for {name}"
                    )
                return {
                    "recipe": scope,
                    "value": value,
                    "provenance": provenance,
                    "unexpanded_value": unexpanded_value,
                    "operations": operations,
                    "active_overrides": active_overrides,
                }
        raise ServerUnavailable(
            f"BitBake server returned an unsupported variable response for {name}"
        )

    def rootfs_sources(self, recipe):
        operation = self.optional_server_operation("get_rootfs_sources")
        if operation is None:
            return {
                "image_manifest": os.environ.get("IMAGE_MANIFEST"),
                "pkgdata_dir": os.environ.get("PKGDATA_DIR"),
                "image_rootfs": os.environ.get("IMAGE_ROOTFS"),
            }
        try:
            response = operation(recipe)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not query rootfs sources for {recipe} from the BitBake server: {exc}"
            )
        keys = ("image_manifest", "pkgdata_dir", "image_rootfs")
        if not isinstance(response, dict) or any(
            response.get(key) is not None and not isinstance(response.get(key), str)
            for key in keys
        ):
            raise ServerUnavailable(
                "BitBake server returned malformed rootfs source data"
            )
        return {key: response.get(key) for key in keys}

    def recipes(self, filter_value):
        operation = self.optional_server_operation("list_recipes")
        if operation is None:
            return None
        try:
            response = operation(filter_value)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not list recipes from the BitBake server: {exc}"
            )
        return typed_recipes(response)

    def layers(self):
        operation = self.optional_server_operation("list_layers")
        if operation is None:
            return None
        try:
            response = operation()
        except Exception as exc:
            raise ServerUnavailable(
                f"could not list layers from the BitBake server: {exc}"
            )
        return typed_layers(response)

    def dependencies(self, recipe):
        """Return server-resolved build and runtime dependencies for one recipe."""
        operation = self.optional_server_operation("get_dependencies")
        if operation is None:
            raise ServerUnavailable(
                "connected BitBake server does not provide get_dependencies; authoritative dependency inspection is unavailable"
            )
        try:
            response = operation(recipe)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect dependencies for {recipe} from the BitBake server: {exc}"
            )
        return typed_dependencies(response)

    def dependency_graph(self, recipe):
        """Return a bounded typed graph from an authoritative server operation."""
        operation = self.optional_server_operation("get_dependency_graph")
        if operation is not None:
            try:
                response = operation(recipe)
            except Exception as exc:
                raise ServerUnavailable(
                    f"could not inspect the dependency graph for {recipe} from the BitBake server: {exc}"
                )
            return typed_dependency_graph(response, recipe)

        dependencies = self.dependencies(recipe)
        edges = [
            {
                "from": {"recipe": recipe},
                "to": {"recipe": dependency},
                "kind": kind,
            }
            for kind, values in (
                ("build", dependencies["build"]),
                ("runtime", dependencies["runtime"]),
            )
            for dependency in values
        ]
        return typed_dependency_graph(
            {
                "root": {"recipe": recipe},
                "nodes": [],
                "edges": edges,
                "limitations": [
                    "Legacy server supplied direct recipe edges only; task dependencies are unavailable."
                ],
            },
            recipe,
        )

    def recipe_sources(self, recipe):
        operation = self.optional_server_operation("get_recipe_sources")
        if operation is None:
            raise ServerUnavailable(
                "connected BitBake server does not provide get_recipe_sources; authoritative recipe metadata paths are unavailable"
            )
        try:
            response = operation(recipe)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect metadata paths for {recipe} from the BitBake server: {exc}"
            )
        if not isinstance(response, list) or not all(
            isinstance(path, str) for path in response
        ):
            raise ServerUnavailable(
                "BitBake server returned malformed recipe source data"
            )
        return response

    def recipe_metadata(self, recipe):
        operation = self.optional_server_operation("get_recipe_metadata")
        if operation is None:
            raise ServerUnavailable(
                "connected BitBake server does not provide get_recipe_metadata; authoritative recipe details are unavailable"
            )
        try:
            return typed_recipe_metadata(operation(recipe))
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect metadata for {recipe} from the BitBake server: {exc}"
            )

    def layer_relationships(self):
        operation = self.optional_server_operation("get_layer_relationships")
        if operation is None:
            raise ServerUnavailable(
                "connected BitBake server does not provide get_layer_relationships; authoritative layer relationships are unavailable"
            )
        try:
            return typed_layer_relationships(operation())
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect layer relationships from the BitBake server: {exc}"
            )

    def native_events(self):
        """Return one bounded event slice without monopolizing command input."""
        if self.connection is None:
            return []
        drain = getattr(self.connection, "drain_events", None)
        if not callable(drain):
            return []
        if self.native_event_iterator is None:
            try:
                drained = drain()
            except Exception as exc:
                return [
                    {
                        "type": "warning",
                        "message": f"could not drain BitBake server events: {exc}",
                    }
                ]
            if drained is None:
                return []
            try:
                self.native_event_iterator = iter(drained)
            except TypeError:
                return [
                    {
                        "type": "warning",
                        "message": "BitBake server drain_events result is not iterable",
                    }
                ]

        events = []
        for _ in range(MAX_NATIVE_EVENTS_PER_POLL):
            try:
                raw = next(self.native_event_iterator)
            except StopIteration:
                self.native_event_iterator = None
                break
            event = normalize_event(raw, self.task_identities_by_pid)
            if not event:
                continue
            kind = event.get("type")
            if kind == "build_completed":
                if not self.build_active:
                    continue
                self.build_active = False
                self.task_identities_by_pid.clear()
                self.native_event_iterator = None
                events.append(event)
                break
            if not self.build_active and kind in {
                "build_started",
                "parse_progress",
                "task_queued",
                "task_started",
                "task_progress",
                "task_completed",
            }:
                continue
            events.append(event)
        return events

    def mock_events(self):
        try:
            raw = json.loads(os.environ.get("YOCTUI_MOCK_EVENTS_JSON", "[]"))
        except json.JSONDecodeError:
            return []
        if not isinstance(raw, list):
            return []
        return [
            event
            for event in (
                normalize_event(item, self.task_identities_by_pid) for item in raw
            )
            if event
        ]


class EnvironmentAdapter(BitBakeAdapter):
    def __init__(self, version=None):
        super().__init__(version)


def select_adapter(version=None, implementation=None):
    module = None
    if version is None:
        try:
            import bb as module  # type: ignore[import-not-found]

            version = getattr(module, "__version__", None)
        except ImportError:
            version = bitbake_version()
    if implementation is None:
        # Backward-compatible bridge clients have no daemon snapshot. Probe
        # the initialized module shape directly; never choose by version.
        return (
            BitBakeAdapter(version, module)
            if module is not None
            else EnvironmentAdapter(version)
        )
    if not (
        implementation.startswith("tinfoil.")
        or implementation.startswith("bitbake.server_socket")
    ):
        raise CompatibilityError(
            f"daemon selected an unsupported BitBake API implementation: {implementation!r}"
        )
    if module is None:
        raise CompatibilityError(
            "daemon selected a BitBake API implementation but the initialized environment does not expose the bb module"
        )
    return BitBakeAdapter(version, module)


def configure_compatibility(command):
    compatibility = command.get("compatibility")
    if compatibility is None:
        adapter = select_adapter()
        return adapter, None, []
    if not isinstance(compatibility, dict):
        raise CompatibilityError("bridge compatibility payload must be an object")
    generation = compatibility.get("generation")
    build_directory = compatibility.get("build_directory")
    capabilities = compatibility.get("capabilities")
    if (
        not isinstance(generation, int)
        or isinstance(generation, bool)
        or generation <= 0
    ):
        raise CompatibilityError("bridge compatibility generation must be positive")
    if not isinstance(build_directory, str) or not os.path.isabs(build_directory):
        raise CompatibilityError(
            "bridge compatibility build directory must be absolute"
        )
    if os.path.realpath(build_directory) != os.path.realpath(os.getcwd()):
        raise CompatibilityError(
            "bridge compatibility belongs to another build directory"
        )
    if not isinstance(capabilities, list) or len(capabilities) > 64:
        raise CompatibilityError(
            "bridge compatibility capability list is invalid or oversized"
        )
    seen = set()
    direct_implementations = {
        "bitbake.workspace_inspection": "tinfoil.workspace",
        "bitbake.recipe_inventory": "tinfoil.recipes",
        "bitbake.recipe_dependencies": "tinfoil.dependencies",
        "bitbake.recipe_sources": "tinfoil.recipe_sources",
        "bitbake.recipe_metadata": "tinfoil.recipe_metadata",
        "bitbake.layer_inventory": "tinfoil.layers",
        "bitbake.layer_relationships": "tinfoil.layer_relationships",
        "bitbake.build": "tinfoil.build",
        "bitbake.cancellation": "tinfoil.cancel",
        "bitbake.task_list": "tinfoil.tasks",
        "bitbake.dependency_graph": "tinfoil.dependency_graph",
        "bitbake.getvar": "tinfoil.getvar",
        "bitbake.variable_history": "tinfoil.variable_history",
        "bitbake.server_socket": "bitbake.server_socket",
        "bitbake.native_events": "tinfoil.native_events",
    }
    for capability in capabilities:
        if (
            not isinstance(capability, dict)
            or not isinstance(capability.get("id"), str)
            or not isinstance(capability.get("implementation"), str)
            or capability["id"] in seen
        ):
            raise CompatibilityError("bridge compatibility capability entry is invalid")
        expected = direct_implementations.get(capability["id"])
        if expected is None or not (
            capability["implementation"] == expected
            or capability["implementation"].startswith("tinfoil.adapter.")
        ):
            raise CompatibilityError(
                f"bridge compatibility implementation does not authorize {capability['id']}"
            )
        seen.add(capability["id"])
    implementation = next(
        (
            item["implementation"]
            for item in capabilities
            if item["implementation"].startswith("tinfoil.adapter.")
        ),
        next((item["implementation"] for item in capabilities), None),
    )
    adapter = select_adapter(implementation=implementation)
    adapter.compatibility_generation = generation
    return adapter, generation, adapter.negotiate(capabilities) if capabilities else []


def workspace_data(version):
    keys = (
        "MACHINE",
        "DISTRO",
        "BBLAYERS",
        "DL_DIR",
        "SSTATE_DIR",
        "TMPDIR",
        "DEPLOY_DIR_IMAGE",
        "PKGDATA_DIR",
        "IMAGE_MANIFEST",
        "IMAGE_ROOTFS",
        "WKS_FILE",
        "WKS_FILES",
        "WKS_SEARCH_PATH",
        "WKS_FILES_DIR",
        "PACKAGE_CLASSES",
        "BB_NUMBER_THREADS",
        "PARALLEL_MAKE",
    )
    variables = {key: os.environ[key] for key in keys if key in os.environ}
    release = os.environ.get("DISTRO_VERSION") or os.environ.get(
        "OECORE_DISTRO_VERSION"
    )
    return {
        "type": "workspace",
        "data": {
            "build_dir": os.environ.get("BUILDDIR", os.getcwd()),
            "source_dir": os.environ.get("COREBASE"),
            "variables": variables,
            "variable_provenance": configured_variable_provenance(),
            "variable_provenance_chain": configured_variable_provenance_chain(),
            "bitbake_version": version,
            "release": release,
            "layers": [],
            "recipes": [],
        },
    }


def configured_variable_provenance():
    """Accept bridge-provided provenance without interpreting Yocto metadata locally."""
    try:
        raw = json.loads(os.environ.get("YOCTUI_VARIABLE_PROVENANCE_JSON", "{}"))
    except json.JSONDecodeError:
        return {}
    if not isinstance(raw, dict):
        return {}
    return {
        name: provenance
        for name, provenance in raw.items()
        if isinstance(name, str) and isinstance(provenance, str)
    }


def configured_variable_provenance_chain():
    try:
        raw = json.loads(os.environ.get("YOCTUI_VARIABLE_PROVENANCE_CHAIN_JSON", "{}"))
    except json.JSONDecodeError:
        return {}
    if not isinstance(raw, dict):
        return {}
    return {
        name: chain
        for name, chain in raw.items()
        if isinstance(name, str)
        and isinstance(chain, list)
        and all(isinstance(source, str) for source in chain)
    }


def typed_workspace(response):
    if not isinstance(response, dict):
        raise ServerUnavailable(
            "BitBake server returned an unsupported workspace response"
        )

    def optional_string(name):
        value = response.get(name)
        if value is None or isinstance(value, str):
            return value
        raise ServerUnavailable(f"BitBake server returned malformed {name} data")

    def string_map(name):
        value = response.get(name, {})
        if isinstance(value, dict) and all(
            isinstance(key, str) and isinstance(item, str)
            for key, item in value.items()
        ):
            return value
        raise ServerUnavailable(f"BitBake server returned malformed {name} data")

    def string_list_map(name):
        value = response.get(name, {})
        if isinstance(value, dict) and all(
            isinstance(key, str)
            and isinstance(items, list)
            and all(isinstance(item, str) for item in items)
            for key, items in value.items()
        ):
            return value
        raise ServerUnavailable(f"BitBake server returned malformed {name} data")

    return {
        "build_dir": optional_string("build_dir"),
        "source_dir": optional_string("source_dir"),
        "variables": string_map("variables"),
        "variable_provenance": string_map("variable_provenance"),
        "variable_provenance_chain": string_list_map("variable_provenance_chain"),
        "bitbake_version": optional_string("bitbake_version"),
        "release": optional_string("release"),
        "layers": typed_layers(response.get("layers", [])),
        "recipes": typed_recipes(response.get("recipes", [])),
    }


def configured_layers():
    values = []
    for path in os.environ.get("BBLAYERS", "").split():
        values.append(
            {
                "name": os.path.basename(path.rstrip("/")) or path,
                "path": path,
                "priority": None,
            }
        )
    return values


def configured_recipes():
    raw = os.environ.get("YOCTUI_RECIPES_JSON", "[]")
    try:
        recipes = json.loads(raw)
        if isinstance(recipes, list) and all(
            isinstance(item, dict) and isinstance(item.get("name"), str)
            for item in recipes
        ):
            return [
                {
                    "name": item["name"],
                    "version": item.get("version"),
                    "layer": item.get("layer"),
                }
                for item in recipes
            ]
    except json.JSONDecodeError:
        pass
    return []


def bitbake_recipes(filter_value):
    """Ask BitBake for its parsed recipe inventory when no server API is available."""
    try:
        result = subprocess.run(
            ["bitbake", "-s"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=120,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None

    if result.returncode != 0:
        return None
    recipes = []
    for line in result.stdout.splitlines():
        match = re.match(r"^([A-Za-z0-9_.+-]+)\s*:\s*(\S+)", line)
        if match and (
            filter_value is None or filter_value.lower() in match.group(1).lower()
        ):
            recipes.append(
                {"name": match.group(1), "version": match.group(2), "layer": None}
            )
    return recipes


def bitbake_layer_recipes(filter_value):
    try:
        result = subprocess.run(
            ["bitbake-layers", "show-recipes"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=120,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    recipes = []
    current = None
    for line in result.stdout.splitlines():
        heading = re.match(r"^([A-Za-z0-9_.+-]+):$", line)
        if heading:
            current = heading.group(1)
            continue
        detail = re.match(r"^\s+([A-Za-z0-9_.+-]+)\s+(\S+)", line)
        if (
            current
            and detail
            and (filter_value is None or filter_value.lower() in current.lower())
        ):
            recipes.append(
                {"name": current, "version": detail.group(2), "layer": detail.group(1)}
            )
            current = None
    return recipes


def bitbake_layers():
    try:
        result = subprocess.run(
            ["bitbake-layers", "show-layers"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=120,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    layers = []
    for line in result.stdout.splitlines():
        match = re.match(r"^(\S+)\s+(\S+)\s+(\d+)\s*$", line)
        if match:
            layers.append(
                {
                    "name": match.group(1),
                    "path": match.group(2),
                    "priority": int(match.group(3)),
                }
            )
    return layers


def typed_recipes(response):
    if not isinstance(response, list):
        raise ServerUnavailable(
            "BitBake server returned an unsupported recipe response"
        )
    if not all(
        isinstance(recipe, dict)
        and isinstance(recipe.get("name"), str)
        and (recipe.get("version") is None or isinstance(recipe.get("version"), str))
        and (recipe.get("layer") is None or isinstance(recipe.get("layer"), str))
        and (
            recipe.get("preferred_version") is None
            or isinstance(recipe.get("preferred_version"), str)
        )
        and (recipe.get("file") is None or isinstance(recipe.get("file"), str))
        and (
            recipe.get("append_count") is None
            or isinstance(recipe.get("append_count"), int)
        )
        for recipe in response
    ):
        raise ServerUnavailable("BitBake server returned malformed recipe data")
    return [
        {
            "name": recipe["name"],
            "version": recipe.get("version"),
            "layer": recipe.get("layer"),
            "preferred_version": recipe.get("preferred_version"),
            "file": recipe.get("file"),
            "append_count": recipe.get("append_count"),
        }
        for recipe in response
    ]


def typed_recipe_metadata(response):
    if not isinstance(response, dict) or not isinstance(response.get("recipe"), str):
        raise ServerUnavailable("BitBake server returned malformed recipe metadata")
    list_fields = ("tasks", "sources", "patches", "packages", "history")
    if any(
        response.get(field) is not None
        and (
            not isinstance(response.get(field), list)
            or not all(isinstance(value, str) for value in response[field])
        )
        for field in list_fields
    ):
        raise ServerUnavailable("BitBake server returned malformed recipe metadata")
    if response.get("workspace_status") not in (None, "clean", "modified"):
        raise ServerUnavailable("BitBake server returned an invalid workspace status")
    if response.get("build_status") not in (
        None,
        "idle",
        "queued",
        "running",
        "succeeded",
        "failed",
        "cancelled",
    ):
        raise ServerUnavailable(
            "BitBake server returned an invalid recipe build status"
        )
    return {
        "recipe": response["recipe"],
        "workspace_status": response.get("workspace_status"),
        "build_status": response.get("build_status"),
        **{field: response.get(field) for field in list_fields},
    }


def typed_layers(response):
    if not isinstance(response, list):
        raise ServerUnavailable("BitBake server returned an unsupported layer response")
    if not all(
        isinstance(layer, dict)
        and isinstance(layer.get("name"), str)
        and isinstance(layer.get("path"), str)
        and (layer.get("priority") is None or isinstance(layer.get("priority"), int))
        for layer in response
    ):
        raise ServerUnavailable("BitBake server returned malformed layer data")
    return [
        {
            "name": layer["name"],
            "path": layer["path"],
            "priority": layer.get("priority"),
        }
        for layer in response
    ]


def typed_dependencies(response):
    if not isinstance(response, dict):
        raise ServerUnavailable(
            "BitBake server returned an unsupported dependency response"
        )
    build = response.get("build", [])
    runtime = response.get("runtime", [])
    if not all(
        isinstance(values, list) and all(isinstance(value, str) for value in values)
        for values in (build, runtime)
    ):
        raise ServerUnavailable("BitBake server returned malformed dependency data")
    return {"build": build, "runtime": runtime}


def dependency_node_id(value):
    if not isinstance(value, dict):
        raise ServerUnavailable(
            "BitBake server returned a malformed dependency identity"
        )
    recipe = value.get("recipe")
    task = value.get("task")
    if (
        not isinstance(recipe, str)
        or not recipe
        or len(recipe) > 512
        or any(ord(character) < 32 for character in recipe)
        or (
            task is not None
            and (
                not isinstance(task, str)
                or not task
                or len(task) > 512
                or any(ord(character) < 32 for character in task)
            )
        )
    ):
        raise ServerUnavailable(
            "BitBake server returned a malformed dependency identity"
        )
    result = {"recipe": recipe}
    if task is not None:
        result["task"] = task
    return result


def dependency_task_id(value):
    if not isinstance(value, str) or "." not in value:
        raise ServerUnavailable("BitBake server returned a malformed task dependency")
    recipe, task = value.rsplit(".", 1)
    return dependency_node_id({"recipe": recipe, "task": task})


def typed_dependency_graph(response, requested_recipe):
    if not isinstance(response, dict):
        raise ServerUnavailable(
            "BitBake server returned an unsupported dependency graph response"
        )
    root = dependency_node_id(response.get("root"))
    if root != {"recipe": requested_recipe}:
        raise ServerUnavailable(
            "BitBake server returned a dependency graph for a different root"
        )
    raw_nodes = response.get("nodes", [])
    raw_edges = response.get("edges", [])
    limitations = response.get("limitations", [])
    if (
        not isinstance(raw_nodes, list)
        or not isinstance(raw_edges, list)
        or not isinstance(limitations, list)
        or not all(
            isinstance(value, str) and len(value) <= 512 for value in limitations
        )
    ):
        raise ServerUnavailable(
            "BitBake server returned malformed dependency graph data"
        )

    normalized_nodes = {}
    dropped_paths = 0
    for raw_node in raw_nodes:
        if not isinstance(raw_node, dict):
            raise ServerUnavailable(
                "BitBake server returned a malformed dependency node"
            )
        identity = dependency_node_id(raw_node.get("id"))
        provider = raw_node.get("provider")
        log = raw_node.get("log")
        if provider is not None and (
            not isinstance(provider, str) or not os.path.isabs(provider)
        ):
            provider = None
            dropped_paths += 1
        if log is not None and (not isinstance(log, str) or not os.path.isabs(log)):
            log = None
            dropped_paths += 1
        key = (identity["recipe"], identity.get("task"))
        candidate = {"id": identity}
        if provider is not None:
            candidate["provider"] = provider
        if log is not None:
            candidate["log"] = log
        previous = normalized_nodes.get(key)
        if previous is None or json.dumps(candidate, sort_keys=True) < json.dumps(
            previous, sort_keys=True
        ):
            normalized_nodes[key] = candidate

    edge_values = set()
    for raw_edge in raw_edges:
        if not isinstance(raw_edge, dict):
            raise ServerUnavailable(
                "BitBake server returned a malformed dependency edge"
            )
        source = dependency_node_id(raw_edge.get("from"))
        destination = dependency_node_id(raw_edge.get("to"))
        kind = raw_edge.get("kind")
        if kind not in ("build", "runtime", "task"):
            raise ServerUnavailable(
                "BitBake server returned an unknown dependency edge kind"
            )
        source_key = (source["recipe"], source.get("task"))
        destination_key = (destination["recipe"], destination.get("task"))
        if source_key == destination_key:
            continue
        edge_values.add((source_key, destination_key, kind))
        normalized_nodes.setdefault(source_key, {"id": source})
        normalized_nodes.setdefault(destination_key, {"id": destination})

    normalized_nodes.setdefault((requested_recipe, None), {"id": root})
    sorted_nodes = [
        normalized_nodes[key]
        for key in sorted(
            normalized_nodes, key=lambda value: (value[0], value[1] or "")
        )
    ]
    dropped_nodes = max(0, len(sorted_nodes) - MAX_DEPENDENCY_NODES)
    sorted_nodes = sorted_nodes[:MAX_DEPENDENCY_NODES]
    retained = {(node["id"]["recipe"], node["id"].get("task")) for node in sorted_nodes}
    if (requested_recipe, None) not in retained:
        sorted_nodes[-1] = {"id": root}
        sorted_nodes.sort(
            key=lambda node: (node["id"]["recipe"], node["id"].get("task") or "")
        )
        retained = {
            (node["id"]["recipe"], node["id"].get("task")) for node in sorted_nodes
        }

    sorted_edges = [
        {
            "from": {
                "recipe": source[0],
                **({"task": source[1]} if source[1] is not None else {}),
            },
            "to": {
                "recipe": destination[0],
                **({"task": destination[1]} if destination[1] is not None else {}),
            },
            "kind": kind,
        }
        for source, destination, kind in sorted(
            edge_values,
            key=lambda value: (
                value[0][0],
                value[0][1] or "",
                value[1][0],
                value[1][1] or "",
                value[2],
            ),
        )
        if source in retained and destination in retained
    ]
    dropped_edges = len(edge_values) - len(sorted_edges)
    if len(sorted_edges) > MAX_DEPENDENCY_EDGES:
        dropped_edges += len(sorted_edges) - MAX_DEPENDENCY_EDGES
        sorted_edges = sorted_edges[:MAX_DEPENDENCY_EDGES]
    limitations = list(dict.fromkeys(limitations))
    if dropped_paths:
        limitations.append(
            f"Dropped {dropped_paths} non-absolute provider or log paths."
        )
    if dropped_nodes or dropped_edges:
        limitations.append(
            f"Dependency graph bounds dropped {dropped_nodes} nodes and {dropped_edges} edges."
        )
    return {
        "root": root,
        "nodes": sorted_nodes,
        "edges": sorted_edges,
        "limitations": limitations,
    }


def dependency_graph_from_deptree(recipe, response):
    if not isinstance(response, dict):
        raise ServerUnavailable("BitBake returned malformed dependency tree data")
    recipes = response.get("pn")
    build_dependencies = response.get("depends")
    runtime_dependencies = response.get("rdepends-pn")
    task_dependencies = response.get("tdepends")
    providers = response.get("providermap", {})
    if not all(
        isinstance(value, dict)
        for value in (
            recipes,
            build_dependencies,
            runtime_dependencies,
            task_dependencies,
            providers,
        )
    ):
        raise ServerUnavailable("BitBake returned incomplete dependency tree data")

    def provider_name(value):
        provider = providers.get(value)
        if (
            isinstance(provider, (list, tuple))
            and provider
            and isinstance(provider[0], str)
        ):
            return provider[0]
        return value

    nodes = []
    edges = []
    for name, metadata in recipes.items():
        if not isinstance(name, str) or not isinstance(metadata, dict):
            raise ServerUnavailable("BitBake returned malformed recipe dependency data")
        provider = metadata.get("filename")
        nodes.append(
            {
                "id": {"recipe": name},
                "provider": provider if isinstance(provider, str) else None,
            }
        )
    for kind, dependencies in (
        ("build", build_dependencies),
        ("runtime", runtime_dependencies),
    ):
        for source, destinations in dependencies.items():
            if not isinstance(source, str) or not isinstance(
                destinations, (list, tuple)
            ):
                raise ServerUnavailable(
                    "BitBake returned malformed recipe dependency edges"
                )
            for destination in destinations:
                if not isinstance(destination, str):
                    raise ServerUnavailable(
                        "BitBake returned malformed recipe dependency edges"
                    )
                edges.append(
                    {
                        "from": {"recipe": source},
                        "to": {"recipe": provider_name(destination)},
                        "kind": kind,
                    }
                )
    for source, destinations in task_dependencies.items():
        source_id = dependency_task_id(source)
        nodes.append({"id": source_id})
        if not isinstance(destinations, (list, tuple)):
            raise ServerUnavailable("BitBake returned malformed task dependency edges")
        for destination in destinations:
            destination_id = dependency_task_id(destination)
            nodes.append({"id": destination_id})
            edges.append({"from": source_id, "to": destination_id, "kind": "task"})
    return typed_dependency_graph(
        {
            "root": {"recipe": recipe},
            "nodes": nodes,
            "edges": edges,
            "limitations": [],
        },
        recipe,
    )


def typed_layer_relationships(response):
    fields = ("compatible", "depends", "overlays", "appends")
    if not isinstance(response, list) or not all(
        isinstance(layer, dict)
        and isinstance(layer.get("name"), str)
        and (layer.get("priority") is None or isinstance(layer.get("priority"), int))
        and all(
            isinstance(layer.get(field, []), list)
            and all(isinstance(value, str) for value in layer.get(field, []))
            for field in fields
        )
        for layer in response
    ):
        raise ServerUnavailable(
            "BitBake server returned malformed layer relationship data"
        )
    return [
        {
            "name": layer["name"],
            "priority": layer.get("priority"),
            **{field: layer.get(field, []) for field in fields},
        }
        for layer in response
    ]


def event_value(event, *names, default=None):
    for name in names:
        value = (
            event.get(name) if isinstance(event, dict) else getattr(event, name, None)
        )
        if value is not None:
            return value
    return default


def normalized_nonnegative_integer(value, maximum=None):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    if isinstance(value, float) and not math.isfinite(value):
        return None
    if value < 0:
        return None
    normalized = int(value)
    return min(normalized, maximum) if maximum is not None else normalized


def normalized_task_stats(event):
    stats = event_value(event, "stats")
    if stats is None:
        return None
    values = {
        name: event_value(stats, name)
        for name in ("completed", "total", "active", "failed")
    }
    if not all(isinstance(value, int) and value >= 0 for value in values.values()):
        return None
    return values


def task_recipe(event):
    recipe = event_value(event, "recipe", "pn")
    if isinstance(recipe, str):
        return recipe
    task_file = event_value(event, "taskfile")
    if not isinstance(task_file, str):
        return None
    stem = os.path.basename(task_file).removesuffix(".bb")
    return re.sub(r"_[0-9].*$", "", stem) or None


def normalize_event(event, task_identities_by_pid=None):
    kind = event_value(event, "type", "event_type")
    if not isinstance(kind, str) and event is not None:
        kind = type(event).__name__
    normalized_kind = kind.lower() if isinstance(kind, str) else None
    recipe = task_recipe(event)
    task = event_value(event, "task", "taskname")
    if normalized_kind in ("buildstarted", "build_started"):
        if task_identities_by_pid is not None:
            task_identities_by_pid.clear()
        return {"type": "build_started"}
    if normalized_kind in ("parsestarted", "parse_started"):
        return {
            "type": "parse_progress",
            "current": 0,
            "total": normalized_nonnegative_integer(event_value(event, "total")),
        }
    if normalized_kind in ("parseprogress", "parse_progress"):
        return {
            "type": "parse_progress",
            "current": normalized_nonnegative_integer(
                event_value(event, "current", "progress")
            ),
            "total": normalized_nonnegative_integer(event_value(event, "total")),
        }
    if normalized_kind in ("processstarted", "process_started"):
        return {
            "type": "parse_progress",
            "current": 0,
            "total": normalized_nonnegative_integer(event_value(event, "total")),
        }
    if normalized_kind in ("processprogress", "process_progress"):
        return {
            "type": "parse_progress",
            "current": normalized_nonnegative_integer(
                event_value(event, "progress"), 100
            ),
            "total": 100,
        }
    if normalized_kind in ("processfinished", "process_finished"):
        return {"type": "parse_progress", "current": 100, "total": 100}
    if normalized_kind in ("parsecompleted", "parse_completed"):
        total = normalized_nonnegative_integer(event_value(event, "total"))
        return {"type": "parse_progress", "current": total, "total": total}
    if normalized_kind in ("buildcompleted", "build_completed"):
        exit_code = event_value(event, "exit_code", "returncode")
        explicit_success = event_value(event, "success")
        failures = event_value(event, "_failures", "failures")
        if failures is None:
            getter = getattr(event, "getFailures", None)
            failures = getter() if callable(getter) else None
        interrupted = event_value(event, "_interrupted", "interrupted", default=0)
        success = (
            bool(explicit_success)
            if explicit_success is not None
            else not bool(failures) and not bool(interrupted)
        )
        if exit_code is None:
            exit_code = 0 if success else 1
        return {
            "type": "build_completed",
            "success": success,
            "exit_code": exit_code if isinstance(exit_code, int) else None,
        }
    if normalized_kind in (
        "tasksucceeded",
        "taskcompleted",
        "task_completed",
        "taskfailed",
        "taskfailedsilent",
    ) and all(isinstance(value, str) for value in (recipe, task)):
        success = normalized_kind not in ("taskfailed", "taskfailedsilent") and bool(
            event_value(event, "success", default=True)
        )
        pid = normalized_nonnegative_integer(event_value(event, "pid"))
        if task_identities_by_pid is not None and pid is not None:
            task_identities_by_pid.pop(pid, None)
        return {
            "type": "task_completed",
            "recipe": recipe,
            "task": task,
            "success": success,
        }
    if normalized_kind in ("taskstarted", "task_started") and all(
        isinstance(value, str) for value in (recipe, task)
    ):
        pid = normalized_nonnegative_integer(event_value(event, "pid"))
        worker = event_value(event, "worker")
        if task_identities_by_pid is not None and pid is not None:
            task_identities_by_pid[pid] = (recipe, task)
        return {
            "type": "task_started",
            "recipe": recipe,
            "task": task,
            "pid": pid,
            "worker": str(worker) if worker is not None else None,
            "log_path": event_value(event, "logfile"),
            "stats": normalized_task_stats(event),
        }
    if normalized_kind in ("runqueuetaskstarted", "scenequeuetaskstarted") and all(
        isinstance(value, str) for value in (recipe, task)
    ):
        return {
            "type": "task_queued",
            "recipe": recipe,
            "task": task,
            "worker": None,
            "stats": normalized_task_stats(event),
        }
    if normalized_kind in ("taskprogress", "task_progress"):
        pid = normalized_nonnegative_integer(event_value(event, "pid"))
        if not all(isinstance(value, str) for value in (recipe, task)):
            identity = (
                task_identities_by_pid.get(pid)
                if task_identities_by_pid is not None and pid is not None
                else None
            )
            if identity is None:
                return None
            recipe, task = identity
        return {
            "type": "task_progress",
            "recipe": recipe,
            "task": task,
            "progress": normalized_nonnegative_integer(
                event_value(event, "progress"), 100
            ),
        }
    message = event_value(event, "message", "msg")
    diagnostic_levels = {
        "warning": "warning",
        "warn": "warning",
        "error": "error",
        "fatal": "error",
    }
    if normalized_kind in ("log", "logrecord", *diagnostic_levels) and isinstance(
        message, str
    ):
        pid = normalized_nonnegative_integer(event_value(event, "pid"))
        if not all(isinstance(value, str) for value in (recipe, task)):
            identity = (
                task_identities_by_pid.get(pid)
                if task_identities_by_pid is not None and pid is not None
                else None
            )
            if identity is not None:
                recipe, task = identity
        level = event_value(
            event,
            "level",
            "levelname",
            default=diagnostic_levels.get(normalized_kind, "info"),
        )
        return {
            "type": "log",
            "level": level.lower() if isinstance(level, str) else "info",
            "message": message,
            "recipe": recipe,
            "task": task,
            "path": event_value(event, "path", "filename"),
        }
    if normalized_kind in ("commandcompleted", "command_completed"):
        return None
    if normalized_kind in ("commandfailed", "commandexit", "command_failed"):
        return {
            "type": "build_completed",
            "success": False,
            "exit_code": 1,
        }
    return {"type": "warning", "message": f"unrecognized BitBake event: {kind!r}"}


def emit_adapter_events(adapter):
    for event in adapter.native_events():
        emit(event, adapter.build_correlation_id)
        if event.get("type") == "build_completed":
            adapter.build_correlation_id = None


def handle(command, correlation_id, adapter):
    kind = command.get("type") if isinstance(command, dict) else None
    required_capabilities = {
        "inspect_workspace": ("bitbake.workspace_inspection",),
        "list_recipes": ("bitbake.recipe_inventory",),
        "list_layers": ("bitbake.layer_inventory",),
        "get_variable": ("bitbake.getvar",),
        "get_rootfs_sources": ("bitbake.getvar",),
        "get_dependencies": ("bitbake.recipe_dependencies",),
        "get_dependency_graph": ("bitbake.dependency_graph",),
        "get_recipe_sources": ("bitbake.recipe_sources",),
        "get_recipe_metadata": ("bitbake.recipe_metadata",),
        "get_layer_relationships": ("bitbake.layer_relationships",),
        "start_build": ("bitbake.build", "bitbake.native_events"),
        "cancel_build": ("bitbake.cancellation",),
        "terminate_server": ("bitbake.server_socket",),
    }
    if adapter.compatibility_generation is not None:
        missing = [
            capability
            for capability in required_capabilities.get(kind, ())
            if capability not in adapter.negotiated_capabilities
        ]
        if missing:
            error(
                "compatibility_unavailable",
                f"{kind} requires negotiated capability {missing[0]}",
                correlation_id,
            )
            return True
    if kind == "hello":
        emit(
            {
                "type": "hello_ack",
                "bitbake_version": adapter.version,
                "compatibility_generation": adapter.compatibility_generation,
                "capabilities": sorted(adapter.negotiated_capabilities),
            },
            correlation_id,
        )
    elif kind == "inspect_workspace":
        try:
            workspace = adapter.workspace()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
            return True
        emit(workspace, correlation_id)
    elif kind == "start_build":
        targets = command.get("targets")
        if (
            not isinstance(targets, list)
            or not targets
            or not all(isinstance(t, str) and t for t in targets)
        ):
            error(
                "invalid_request",
                "start_build requires non-empty string targets",
                correlation_id,
            )
        else:
            try:
                force = command.get("force", False)
                if not isinstance(force, bool):
                    error(
                        "invalid_request",
                        "start_build force must be a boolean",
                        correlation_id,
                    )
                    return True
                native_events = adapter.start_build(targets, command.get("task"), force)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                adapter.build_correlation_id = correlation_id
                if not native_events:
                    emit({"type": "build_started"}, correlation_id)
                for event in adapter.mock_events():
                    emit(event, correlation_id)
    elif kind == "list_recipes":
        filter_value = command.get("filter")
        if filter_value is not None and not isinstance(filter_value, str):
            error(
                "invalid_request",
                "list_recipes filter must be a string",
                correlation_id,
            )
            return True
        try:
            recipes = adapter.recipes(filter_value)
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
            return True
        if recipes is None:
            recipes = bitbake_layer_recipes(filter_value)
            if recipes is None:
                recipes = bitbake_recipes(filter_value)
            if recipes is None:
                recipes = configured_recipes()
                if filter_value is not None:
                    recipes = [
                        recipe
                        for recipe in recipes
                        if filter_value.lower() in recipe["name"].lower()
                    ]
        emit({"type": "recipes", "recipes": recipes}, correlation_id)
    elif kind == "list_layers":
        try:
            layers = adapter.layers()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
            return True
        if layers is None:
            layers = bitbake_layers()
        emit(
            {
                "type": "layers",
                "layers": configured_layers() if layers is None else layers,
            },
            correlation_id,
        )
    elif kind == "get_variable":
        name = command.get("name")
        recipe = command.get("recipe")
        if (
            not isinstance(name, str)
            or not name
            or (recipe is not None and not isinstance(recipe, str))
        ):
            error(
                "invalid_request",
                "get_variable requires a variable name and optional recipe name",
                correlation_id,
            )
        else:
            try:
                variable = adapter.variable(name, recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
                return True
            if variable is None:
                variable = {
                    "value": os.environ.get(name),
                    "provenance": configured_variable_provenance().get(name),
                }
            emit(
                {
                    "type": "variable",
                    "name": name,
                    "recipe": recipe,
                    **variable,
                },
                correlation_id,
            )
    elif kind == "get_rootfs_sources":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not re.fullmatch(r"[A-Za-z0-9_.+-]+", recipe):
            error(
                "invalid_request",
                "get_rootfs_sources requires an exact image recipe name",
                correlation_id,
            )
        else:
            try:
                sources = adapter.rootfs_sources(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {"type": "rootfs_sources", "recipe": recipe, **sources},
                    correlation_id,
                )
    elif kind == "get_dependency_graph":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not recipe:
            error(
                "invalid_request",
                "get_dependency_graph requires a recipe name",
                correlation_id,
            )
        else:
            try:
                graph = adapter.dependency_graph(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {
                        "type": "dependency_graph",
                        "data": graph,
                    },
                    correlation_id,
                )
    elif kind == "get_dependencies":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not recipe:
            error(
                "invalid_request",
                "get_dependencies requires a recipe name",
                correlation_id,
            )
        else:
            try:
                dependencies = adapter.dependencies(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {
                        "type": "dependencies",
                        "recipe": recipe,
                        **dependencies,
                    },
                    correlation_id,
                )
    elif kind == "get_recipe_sources":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not recipe:
            error(
                "invalid_request",
                "get_recipe_sources requires a recipe name",
                correlation_id,
            )
        else:
            try:
                paths = adapter.recipe_sources(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {"type": "recipe_sources", "recipe": recipe, "paths": paths},
                    correlation_id,
                )
    elif kind == "get_recipe_metadata":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not recipe:
            error(
                "invalid_request",
                "get_recipe_metadata requires a recipe name",
                correlation_id,
            )
        else:
            try:
                metadata = adapter.recipe_metadata(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {"type": "recipe_metadata", "data": metadata},
                    correlation_id,
                )
    elif kind == "get_layer_relationships":
        try:
            layers = adapter.layer_relationships()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            emit({"type": "layer_relationships", "layers": layers}, correlation_id)
    elif kind == "cancel_build":
        try:
            adapter.cancel_build()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            # The cooker shutdown command acknowledges cancellation, but some
            # BitBake releases do not subsequently deliver BuildCompleted to
            # this Tinfoil event stream.  The accepted shutdown request is the
            # authoritative terminal boundary for Yoctui.  Stop polling here
            # so delayed native records cannot resurrect the cancelled build;
            # the daemon then closes the server through terminate_server.
            build_correlation_id = adapter.build_correlation_id or correlation_id
            adapter.build_active = False
            adapter.native_event_iterator = None
            adapter.task_identities_by_pid.clear()
            emit(
                {"type": "build_completed", "success": False, "exit_code": 1},
                build_correlation_id,
            )
            adapter.build_correlation_id = None
    elif kind == "shutdown":
        emit({"type": "bridge_shutdown"}, correlation_id)
        return False
    elif kind == "terminate_server":
        try:
            adapter.server().terminate_server()
        except (ServerUnavailable, RuntimeError) as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            adapter.connection = None
            adapter.build_active = False
            emit({"type": "server_terminated"}, correlation_id)
            return False
    else:
        error("unknown_command", f"unknown command: {kind!r}", correlation_id)
    return True


def main():
    isolate_protocol_output()
    adapter = select_adapter()
    selector = selectors.DefaultSelector()
    selector.register(sys.stdin.buffer, selectors.EVENT_READ)
    try:
        while True:
            # Poll quickly during builds and occasionally while idle so native
            # events cannot be stranded between adjacent client commands.
            ready = selector.select(0.1 if adapter.build_active else 1.0)
            if not ready:
                if adapter.build_active:
                    emit_adapter_events(adapter)
                continue
            raw = sys.stdin.buffer.readline()
            if not raw:
                return
            if len(raw) > MAX_LINE_BYTES:
                error("message_too_large", f"limit is {MAX_LINE_BYTES} bytes")
                continue
            try:
                data = json.loads(raw.decode("utf-8"))
                if data.get("protocol_version") != VERSION:
                    error(
                        "version_mismatch",
                        f"supported version is {VERSION}",
                        data.get("correlation_id"),
                    )
                    continue
                message = data.get("message")
                if isinstance(message, dict) and message.get("type") == "hello":
                    try:
                        adapter.shutdown()
                        adapter, generation, capabilities = configure_compatibility(
                            message
                        )
                    except (CompatibilityError, ServerUnavailable) as exc:
                        error(
                            "compatibility_negotiation_failed",
                            str(exc),
                            data.get("correlation_id"),
                        )
                    else:
                        emit(
                            {
                                "type": "hello_ack",
                                "bitbake_version": adapter.version,
                                "compatibility_generation": generation,
                                "capabilities": capabilities,
                            },
                            data.get("correlation_id"),
                        )
                    continue
                if not handle(message, data.get("correlation_id"), adapter):
                    return
                if adapter.build_active:
                    emit_adapter_events(adapter)
            except (UnicodeDecodeError, json.JSONDecodeError, AttributeError) as exc:
                error("malformed_command", str(exc))
    finally:
        selector.close()
        try:
            adapter.shutdown()
        except Exception as exc:
            print(f"bridge shutdown warning: {exc}", file=sys.stderr)


if __name__ == "__main__":
    main()
