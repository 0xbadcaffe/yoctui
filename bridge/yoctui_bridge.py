#!/usr/bin/env python3
"""NDJSON BitBake bridge. Diagnostics are deliberately written only to stderr."""

import json
import os
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


class BitBakeAdapter:
    def __init__(self, version, family, module=None):
        self.version = version
        self.family = family
        self.module = module
        self.connection = None

    def workspace(self):
        return workspace_data(self.version)

    def server(self):
        if self.connection is not None:
            return self.connection
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
        operation(targets, task)

    def cancel_build(self):
        connection = self.server()
        operation = getattr(connection, "cancel_build", None)
        if not callable(operation):
            raise ServerUnavailable(
                "connected BitBake server does not provide cancel_build"
            )
        operation()

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
            return [
                event for event in (normalize_event(item) for item in events) if event
            ]
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


def event_value(event, *names, default=None):
    for name in names:
        value = (
            event.get(name) if isinstance(event, dict) else getattr(event, name, None)
        )
        if value is not None:
            return value
    return default


def normalize_event(event):
    kind = event_value(event, "type", "event_type")
    if not isinstance(kind, str) and event is not None:
        kind = type(event).__name__
    normalized_kind = kind.lower() if isinstance(kind, str) else None
    recipe = event_value(event, "recipe", "pn")
    task = event_value(event, "task", "taskname")
    if normalized_kind in ("buildstarted", "build_started"):
        return {"type": "build_started"}
    if normalized_kind in ("parseprogress", "parse_progress"):
        return {
            "type": "parse_progress",
            "current": event_value(event, "current"),
            "total": event_value(event, "total"),
        }
    if normalized_kind in ("buildcompleted", "build_completed"):
        return {
            "type": "build_completed",
            "success": bool(event_value(event, "success")),
        }
    if normalized_kind in (
        "tasksucceeded",
        "taskcompleted",
        "task_completed",
        "taskfailed",
    ) and all(isinstance(value, str) for value in (recipe, task)):
        success = normalized_kind not in ("taskfailed",) and bool(
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
        return {
            "type": "task_started",
            "recipe": recipe,
            "task": task,
            "pid": event_value(event, "pid"),
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
    if normalized_kind in ("log", "logrecord") and isinstance(message, str):
        return {
            "type": "log",
            "level": event_value(event, "level", "levelname", default="info"),
            "message": message,
            "recipe": recipe,
            "task": task,
            "path": event_value(event, "path", "filename"),
        }
    return {"type": "warning", "message": f"unrecognized BitBake event: {kind!r}"}


def handle(command, correlation_id, adapter):
    kind = command.get("type") if isinstance(command, dict) else None
    if kind == "hello":
        emit({"type": "hello_ack", "bitbake_version": adapter.version}, correlation_id)
    elif kind == "inspect_workspace":
        emit(adapter.workspace(), correlation_id)
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
                adapter.start_build(targets, command.get("task"))
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit({"type": "build_started"}, correlation_id)
                for event in adapter.native_events():
                    emit(event, correlation_id)
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
    elif kind == "cancel_build":
        try:
            adapter.cancel_build()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            emit({"type": "build_completed", "success": False}, correlation_id)
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
    for raw in sys.stdin.buffer:
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


if __name__ == "__main__":
    main()
