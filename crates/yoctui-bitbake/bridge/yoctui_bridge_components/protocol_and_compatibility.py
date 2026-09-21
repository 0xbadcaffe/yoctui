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
MAX_RECIPE_CHUNK_BYTES = 512 * 1024
MAX_RECIPE_INVENTORY_BYTES = 3 * 1024 * 1024
MAX_RECIPE_INVENTORY_RECORDS = 16384
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


def emit_recipe_inventory(recipes, correlation_id, chunked):
    """Opt-in chunks; preflight the complete inventory before emitting any part."""
    if len(recipes) > MAX_RECIPE_INVENTORY_RECORDS:
        error(
            "recipe_inventory_limit",
            "recipe inventory exceeds 16384 records",
            correlation_id,
        )
        return
    sizes = [
        len(
            json.dumps(recipe, ensure_ascii=False, separators=(",", ":")).encode(
                "utf-8"
            )
        )
        for recipe in recipes
    ]
    if sum(sizes) + max(0, len(recipes) - 1) + 2 > MAX_RECIPE_INVENTORY_BYTES:
        error(
            "recipe_inventory_limit", "recipe inventory exceeds 3 MiB", correlation_id
        )
        return
    if not chunked:
        message = {"type": "recipes", "recipes": recipes}
        frame = {
            "protocol_version": VERSION,
            "sequence": sequence + 1,
            "correlation_id": correlation_id,
            "message": message,
        }
        if (
            len(
                json.dumps(frame, ensure_ascii=False, separators=(",", ":")).encode(
                    "utf-8"
                )
            )
            > MAX_LINE_BYTES
        ):
            error(
                "recipe_inventory_limit",
                "recipe inventory requires chunked transfer",
                correlation_id,
            )
        else:
            emit(message, correlation_id)
        return
    # Reserve space for envelope, chunk fields and the request correlation.
    overhead = (
        len(json.dumps(correlation_id, ensure_ascii=False).encode("utf-8")) + 1024
    )
    budget = MAX_RECIPE_CHUNK_BYTES - overhead
    if budget < 3 or any(size + 3 > budget for size in sizes):
        error(
            "recipe_inventory_limit",
            "recipe record exceeds bounded chunk size",
            correlation_id,
        )
        return
    offset = 0
    while offset < len(recipes) or not recipes:
        end = offset
        used = 2
        while end < len(recipes) and used + sizes[end] + 1 <= budget:
            used += sizes[end] + 1
            end += 1
        emit(
            {
                "type": "recipes_chunk",
                "offset": offset,
                "total": len(recipes),
                "complete": end == len(recipes),
                "recipes": recipes[offset:end],
            },
            correlation_id,
        )
        if end == len(recipes):
            break
        offset = end


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


def tinfoil_probe_capabilities(tinfoil):
    """Inspect actual API endpoints after a read-only server ping; never run work."""
    if (
        not callable(getattr(tinfoil, "run_command", None))
        or tinfoil.run_command("ping") != "Still alive!"
    ):
        raise CompatibilityError(
            "BitBake capability probe could not verify server ping"
        )
    commands = importlib.import_module("bb.command")
    sync = commands.CommandsSync
    asynchronous = commands.CommandsAsync

    def methods(obj, *names):
        return all(callable(getattr(obj, name, None)) for name in names)

    data = getattr(tinfoil, "config_data", None)
    connection = getattr(tinfoil, "server_connection", None)
    events = (
        methods(tinfoil, "set_event_mask", "wait_event")
        and methods(sync, "setEventMask")
        and methods(getattr(connection, "events", None), "waitEvent")
    )
    metadata = methods(tinfoil, "parse_recipes", "parse_recipe") and methods(
        sync, "parseRecipeFile"
    )
    layers = methods(data, "getVar") and methods(sync, "getLayerPriorities")
    available = {
        "workspace": layers,
        "layers": layers,
        "layer_relationships": layers,
        "recipes": layers
        and methods(tinfoil, "parse_recipes")
        and methods(sync, "getRecipes", "getRecipeVersions"),
        "recipe_dependencies": metadata and methods(data, "getVar"),
        "recipe_sources": metadata and methods(tinfoil, "get_file_appends"),
        "recipe_metadata": metadata
        and methods(tinfoil, "get_file_appends", "parse_recipe_file"),
        "tasks": metadata and methods(tinfoil, "get_file_appends", "parse_recipe_file"),
        "build": events
        and methods(data, "getVar")
        and methods(asynchronous, "buildTargets"),
        "cancel": methods(sync, "stateForceShutdown"),
        "variable_history": metadata and methods(data, "getVar"),
        "native_events": events,
        "server_socket": methods(
            getattr(connection, "connection", None), "terminateServer"
        ),
    }
    return sorted(name for name, present in available.items() if present)


def probe_backend_capabilities():
    """One-shot daemon startup probe, separate from the normal NDJSON protocol."""
    module = importlib.import_module("bb")
    connection = TinfoilConnection(module)
    try:
        capabilities = tinfoil_probe_capabilities(connection.tinfoil)
        build_directory = connection.tinfoil.config_data.getVar("TOPDIR")
        if not build_directory or not os.path.isabs(build_directory):
            raise CompatibilityError("BitBake did not report an absolute TOPDIR")
        return {
            "schema": "yoctui.bridge-capability-probe.v1",
            "build_directory": os.path.realpath(build_directory),
            "bitbake_version": getattr(module, "__version__", None),
            "capabilities": capabilities,
        }
    finally:
        connection.shutdown()
