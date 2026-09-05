#!/usr/bin/env python3
"""Deterministic offline BitBake-like bridge used only by performance gates."""

from __future__ import annotations

import json
import os
import selectors
import sys
import time


PROTOCOL_VERSION = 1
MAX_EVENTS_PER_SLICE = 256
sequence = 0
active = False
build_correlation: str | None = None
started_at = 0.0
emitted_events = 0


def emit(message: dict[str, object], correlation: str | None = None) -> None:
    global sequence
    sequence += 1
    envelope: dict[str, object] = {
        "protocol_version": PROTOCOL_VERSION,
        "sequence": sequence,
        "message": message,
    }
    if correlation is not None:
        envelope["correlation_id"] = correlation
    sys.stdout.write(json.dumps(envelope, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def event_for(index: int) -> dict[str, object]:
    task_index = index // 7
    recipe = f"fixture-recipe-{task_index % 128:03d}"
    task = f"do_fixture_{task_index % 16:02d}"
    profile = os.environ.get("YOCTUI_PERF_EVENT_PROFILE", "balanced")
    if profile == "log-heavy":
        if index % 32 == 0:
            return {"type": "warning", "message": f"fixture warning {index}"}
        return {
            "type": "log",
            "level": "info",
            "message": f"fixture ordinary log {index}",
            "recipe": recipe,
            "task": task,
            "path": None,
        }
    if profile == "task-heavy":
        kind = index % 4
        if kind == 0:
            return {"type": "task_queued", "recipe": recipe, "task": task}
        if kind == 1:
            return {
                "type": "task_started",
                "recipe": recipe,
                "task": task,
                "pid": 10_000 + task_index % 1_000,
            }
        if kind == 2:
            return {
                "type": "task_progress",
                "recipe": recipe,
                "task": task,
                "progress": (task_index * 7) % 100,
            }
        return {
            "type": "task_completed",
            "recipe": recipe,
            "task": task,
            "success": True,
        }
    if profile != "balanced":
        raise RuntimeError(f"unknown YOCTUI_PERF_EVENT_PROFILE {profile!r}")
    kind = index % 7
    if kind == 0:
        return {"type": "task_queued", "recipe": recipe, "task": task}
    if kind == 1:
        return {
            "type": "task_started",
            "recipe": recipe,
            "task": task,
            "pid": 10_000 + task_index % 1_000,
        }
    if kind == 2:
        return {
            "type": "task_progress",
            "recipe": recipe,
            "task": task,
            "progress": (task_index * 7) % 100,
        }
    if kind == 3:
        return {
            "type": "log",
            "level": "info",
            "message": f"fixture ordinary log {index}",
            "recipe": recipe,
            "task": task,
            "path": None,
        }
    if kind == 4:
        return {
            "type": "task_progress",
            "recipe": recipe,
            "task": task,
            "progress": (task_index * 7 + 1) % 100,
        }
    if kind == 5:
        return {
            "type": "warning",
            "message": f"fixture warning {index}",
        }
    return {
        "type": "task_completed",
        "recipe": recipe,
        "task": task,
        "success": True,
    }


def emit_due_events() -> None:
    global active, emitted_events
    rate = int(os.environ.get("YOCTUI_PERF_EVENT_RATE", "2000"))
    duration = int(os.environ.get("YOCTUI_PERF_EVENT_DURATION", "90"))
    elapsed = time.monotonic() - started_at
    due = min(int(elapsed * rate) - emitted_events, MAX_EVENTS_PER_SLICE)
    for _ in range(max(0, due)):
        emit(event_for(emitted_events), build_correlation)
        emitted_events += 1
    if elapsed >= duration:
        emit({"type": "build_completed", "success": True, "exit_code": 0}, build_correlation)
        active = False


def handle(message: dict[str, object], correlation: str | None) -> bool:
    global active, build_correlation, emitted_events, started_at
    kind = message.get("type")
    if kind == "hello":
        compatibility = message.get("compatibility")
        generation = None
        capabilities: list[str] = []
        if isinstance(compatibility, dict):
            generation = compatibility.get("generation")
            offered = compatibility.get("capabilities", [])
            if isinstance(offered, list):
                capabilities = [
                    entry["id"]
                    for entry in offered
                    if isinstance(entry, dict) and isinstance(entry.get("id"), str)
                ]
        emit(
            {
                "type": "hello_ack",
                "bitbake_version": "2.18.0-fixture",
                "compatibility_generation": generation,
                "capabilities": capabilities,
            },
            correlation,
        )
    elif kind == "inspect_workspace":
        emit(
            {
                "type": "workspace",
                "data": {
                    "build_dir": os.getcwd(),
                    "source_dir": None,
                    "variables": {},
                    "variable_provenance": {},
                    "variable_provenance_chain": {},
                    "bitbake_version": "2.18.0-fixture",
                    "release": "fixture-only",
                    "layers": [],
                    "recipes": [],
                },
            },
            correlation,
        )
    elif kind == "list_recipes":
        emit({"type": "recipes", "recipes": []}, correlation)
    elif kind == "list_layers":
        emit({"type": "layers", "layers": []}, correlation)
    elif kind == "start_build":
        active = True
        build_correlation = correlation
        emitted_events = 0
        started_at = time.monotonic()
        emit({"type": "build_started"}, correlation)
    elif kind == "cancel_build":
        active = False
        emit(
            {"type": "build_completed", "success": False, "exit_code": 130},
            build_correlation or correlation,
        )
        build_correlation = None
    elif kind == "terminate_server":
        emit({"type": "server_terminated"}, correlation)
        return False
    elif kind == "shutdown":
        emit({"type": "bridge_shutdown"}, correlation)
        return False
    else:
        emit(
            {
                "type": "command_failed",
                "code": "fixture_unsupported",
                "message": f"fixture does not implement {kind!r}",
            },
            correlation,
        )
    return True


def main() -> int:
    selector = selectors.DefaultSelector()
    selector.register(sys.stdin.buffer, selectors.EVENT_READ)
    while True:
        ready = selector.select(0.001 if active else 1.0)
        if active:
            emit_due_events()
        if not ready:
            continue
        raw = sys.stdin.buffer.readline()
        if not raw:
            return 0
        envelope = json.loads(raw)
        message = envelope.get("message")
        if not isinstance(message, dict):
            continue
        if not handle(message, envelope.get("correlation_id")):
            return 0


if __name__ == "__main__":
    raise SystemExit(main())
