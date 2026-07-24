#!/usr/bin/env python3
"""NDJSON BitBake bridge. Diagnostics are deliberately written only to stderr."""

import importlib
import json
import os
import re
import selectors
import subprocess
import sys

VERSION = 1
MAX_LINE_BYTES = 1024 * 1024
sequence = 0


def emit(message, correlation_id=None):
    global sequence
    sequence += 1
    value = {"protocol_version": VERSION, "sequence": sequence, "message": message}
    if correlation_id is not None:
        value["correlation_id"] = correlation_id
    sys.stdout.write(
        json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n"
    )
    sys.stdout.flush()


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

    def _variable_provenance(self, datastore, name):
        try:
            history = datastore.varhistory.variable(name) or []
        except Exception:
            return None
        sources = []
        for item in history:
            if not isinstance(item, dict) or "flag" in item:
                continue
            path = item.get("file")
            line = item.get("line")
            if isinstance(path, str):
                sources.append(f"{path}:{line}" if isinstance(line, int) else path)
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
        layers = self._layers()
        result = []
        for name, paths in recipes:
            if filter_value is not None and filter_value.lower() not in name.lower():
                continue
            recipe_paths = sorted(path for path in paths if isinstance(path, str))
            path = recipe_paths[0] if recipe_paths else None
            version_data = versions.get(path) if path is not None else None
            version = (
                str(version_data[1])
                if isinstance(version_data, (list, tuple)) and len(version_data) > 1
                else None
            )
            result.append(
                {
                    "name": str(name),
                    "version": version,
                    "layer": self._layer_for_path(path, layers)
                    if path is not None
                    else None,
                }
            )
        return result

    def get_variable(self, name, recipe):
        datastore = self.tinfoil.config_data
        if recipe is not None:
            self._ensure_recipes()
            datastore = self.tinfoil.parse_recipe(recipe)
        value = datastore.getVar(name)
        return {
            "value": None if value is None else str(value),
            "provenance": self._variable_provenance(datastore, name),
        }

    def get_dependencies(self, recipe):
        self._ensure_recipes()
        datastore = self.tinfoil.parse_recipe(recipe)
        build = (datastore.getVar("DEPENDS") or "").split()
        runtime = (
            datastore.getVar(f"RDEPENDS:{recipe}") or datastore.getVar("RDEPENDS") or ""
        ).split()
        return {"build": build, "runtime": runtime}

    def get_recipe_sources(self, recipe):
        self._ensure_recipes()
        recipe_file = self.tinfoil.get_recipe_file(recipe)
        appends = self.tinfoil.get_file_appends(recipe_file) or []
        return [recipe_file, *appends]

    def start_build(self, targets, task):
        if self.active:
            raise RuntimeError("a BitBake build is already active")
        self._reset_for_build()
        self.tinfoil.set_event_mask(self.EVENT_MASK)
        selected_task = task or self.tinfoil.config_data.getVar("BB_DEFAULT_TASK")
        self.active = True
        try:
            self.tinfoil.run_command(
                "buildTargets", targets, selected_task, handle_events=False
            )
        except Exception:
            self.active = False
            raise

    def cancel_build(self):
        if not self.active:
            raise RuntimeError("no BitBake build is active")
        self.tinfoil.run_command("stateShutdown", handle_events=False)

    def drain_events(self):
        events = []
        first = True
        while True:
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
        return events

    def shutdown(self):
        if self.tinfoil is not None:
            self.tinfoil.shutdown()
            self.tinfoil = None
        self.active = False


class BitBakeAdapter:
    def __init__(self, version, family, module=None):
        self.version = version
        self.family = family
        self.module = module
        self.connection = None
        self.build_correlation_id = None
        self.build_active = False

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

    def start_build(self, targets, task):
        connection = self.server()
        operation = getattr(connection, "start_build", None)
        if not callable(operation):
            raise ServerUnavailable(
                "connected BitBake server does not provide start_build"
            )
        try:
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
            if (value is None or isinstance(value, str)) and (
                provenance is None or isinstance(provenance, str)
            ):
                return {"value": value, "provenance": provenance}
        raise ServerUnavailable(
            f"BitBake server returned an unsupported variable response for {name}"
        )

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
        """Drain a non-blocking server event hook when the adapter exposes one."""
        if self.connection is None:
            return []
        drain = getattr(self.connection, "drain_events", None)
        if not callable(drain):
            return []
        try:
            events = drain()
        except Exception as exc:
            return [
                {
                    "type": "warning",
                    "message": f"could not drain BitBake server events: {exc}",
                }
            ]
        if events is None:
            return []
        try:
            events = [
                event for event in (normalize_event(item) for item in events) if event
            ]
            if any(event.get("type") == "build_completed" for event in events):
                self.build_active = False
            return events
        except TypeError:
            return [
                {
                    "type": "warning",
                    "message": "BitBake server drain_events result is not iterable",
                }
            ]

    def mock_events(self):
        try:
            raw = json.loads(os.environ.get("YOCTUI_MOCK_EVENTS_JSON", "[]"))
        except json.JSONDecodeError:
            return []
        if not isinstance(raw, list):
            return []
        return [event for event in (normalize_event(item) for item in raw) if event]


class EnvironmentAdapter(BitBakeAdapter):
    def __init__(self):
        super().__init__(None, "environment")


def select_adapter(version=None):
    module = None
    if version is None:
        try:
            import bb as module  # type: ignore[import-not-found]

            version = getattr(module, "__version__", None)
        except ImportError:
            version = bitbake_version()
    if version is None:
        return EnvironmentAdapter()
    try:
        major = int(version.split(".", 1)[0])
    except (AttributeError, ValueError):
        raise CompatibilityError(f"unrecognized BitBake version: {version!r}")
    if major < 1:
        raise CompatibilityError(f"unsupported BitBake version: {version}")
    return BitBakeAdapter(version, "legacy" if major == 1 else "modern", module)


def workspace_data(version):
    keys = (
        "MACHINE",
        "DISTRO",
        "BBLAYERS",
        "DL_DIR",
        "SSTATE_DIR",
        "TMPDIR",
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
        for recipe in response
    ):
        raise ServerUnavailable("BitBake server returned malformed recipe data")
    return [
        {
            "name": recipe["name"],
            "version": recipe.get("version"),
            "layer": recipe.get("layer"),
        }
        for recipe in response
    ]


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


def normalize_event(event):
    kind = event_value(event, "type", "event_type")
    if not isinstance(kind, str) and event is not None:
        kind = type(event).__name__
    normalized_kind = kind.lower() if isinstance(kind, str) else None
    recipe = task_recipe(event)
    task = event_value(event, "task", "taskname")
    if normalized_kind in ("buildstarted", "build_started"):
        return {"type": "build_started"}
    if normalized_kind in ("parsestarted", "parse_started"):
        return {
            "type": "parse_progress",
            "current": 0,
            "total": event_value(event, "total"),
        }
    if normalized_kind in (
        "parseprogress",
        "parse_progress",
        "processprogress",
        "process_progress",
    ):
        return {
            "type": "parse_progress",
            "current": event_value(event, "current", "progress"),
            "total": event_value(event, "total"),
        }
    if normalized_kind in ("parsecompleted", "parse_completed"):
        total = event_value(event, "total")
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
        return {
            "type": "task_completed",
            "recipe": recipe,
            "task": task,
            "success": success,
        }
    if normalized_kind in ("taskstarted", "task_started") and all(
        isinstance(value, str) for value in (recipe, task)
    ):
        pid = event_value(event, "pid")
        worker = event_value(event, "worker")
        return {
            "type": "task_started",
            "recipe": recipe,
            "task": task,
            "pid": pid if isinstance(pid, int) and pid >= 0 else None,
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
    if normalized_kind in ("taskprogress", "task_progress") and all(
        isinstance(value, str) for value in (recipe, task)
    ):
        return {
            "type": "task_progress",
            "recipe": recipe,
            "task": task,
            "progress": event_value(event, "progress"),
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
    if kind == "hello":
        emit({"type": "hello_ack", "bitbake_version": adapter.version}, correlation_id)
    elif kind == "inspect_workspace":
        try:
            workspace = adapter.workspace()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
            return True
        emit(workspace, correlation_id)
    elif kind == "start_build":
        targets = command.get("targets")
        if not isinstance(targets, list) or not all(
            isinstance(t, str) and t for t in targets
        ):
            error(
                "invalid_request",
                "start_build requires non-empty string targets",
                correlation_id,
            )
        else:
            try:
                native_events = adapter.start_build(targets, command.get("task"))
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                adapter.build_correlation_id = correlation_id
                if not native_events:
                    emit({"type": "build_started"}, correlation_id)
                emit_adapter_events(adapter)
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
                    **variable,
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
    elif kind == "get_layer_relationships":
        try:
            layers = adapter.layer_relationships()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            emit({"type": "layer_relationships", "layers": layers}, correlation_id)
    elif kind == "cancel_build":
        try:
            native_events = adapter.cancel_build()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            if not native_events:
                emit({"type": "build_completed", "success": False}, correlation_id)
                adapter.build_active = False
                adapter.build_correlation_id = None
    elif kind == "shutdown":
        emit({"type": "bridge_shutdown"}, correlation_id)
        return False
    else:
        error("unknown_command", f"unknown command: {kind!r}", correlation_id)
    return True


def main():
    try:
        adapter = select_adapter()
    except CompatibilityError as exc:
        error("unsupported_bitbake", str(exc))
        return
    selector = selectors.DefaultSelector()
    selector.register(sys.stdin.buffer, selectors.EVENT_READ)
    try:
        while True:
            # Poll quickly during builds and occasionally while idle so native
            # events cannot be stranded between adjacent client commands.
            ready = selector.select(0.1 if adapter.build_active else 1.0)
            if not ready:
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
                if not handle(data.get("message"), data.get("correlation_id"), adapter):
                    return
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
