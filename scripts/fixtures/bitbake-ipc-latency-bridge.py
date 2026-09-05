#!/usr/bin/env python3
"""Deterministic bridge fixture for saturated daemon IPC latency measurements."""

from __future__ import annotations

import json
import os
import selectors
import sys
import time


PROTOCOL_VERSION = 1
EVENT_PREFIX = "PERF_IPC_LATENCY"

sequence = 0
active = False
build_correlation: str | None = None
next_event = 1
event_interval = 0.002


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


def emit_latency_event() -> None:
    global next_event
    emitted_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    emit(
        {
            "type": "log",
            "level": "info",
            "message": f"{EVENT_PREFIX}:{next_event}:{emitted_ns}",
            "recipe": "ipc-latency-fixture",
            "task": "do_measure",
            "path": None,
        },
        build_correlation,
    )
    next_event += 1


def handle(message: dict[str, object], correlation: str | None) -> bool:
    global active, build_correlation, next_event
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
                "bitbake_version": "2.18.0-ipc-latency-fixture",
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
                    "bitbake_version": "2.18.0-ipc-latency-fixture",
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
        next_event = 1
        emit({"type": "build_started"}, build_correlation)
    elif kind == "cancel_build":
        active = False
        emit(
            {"type": "warning", "message": "PERF_IPC_CANCELLATION_ACK"},
            build_correlation,
        )
        emit(
            {"type": "build_completed", "success": False, "exit_code": 130},
            build_correlation,
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
    global event_interval
    rate = int(os.environ.get("YOCTUI_PERF_IPC_EVENT_RATE", "500"))
    if rate < 100 or rate > 10_000:
        print("YOCTUI_PERF_IPC_EVENT_RATE must be within 100..10000", file=sys.stderr)
        return 2
    event_interval = 1.0 / rate
    selector = selectors.DefaultSelector()
    selector.register(sys.stdin.buffer, selectors.EVENT_READ)
    next_due = time.monotonic()
    while True:
        now = time.monotonic()
        timeout = max(0.0, min(1.0, next_due - now)) if active else 1.0
        ready = selector.select(timeout)
        now = time.monotonic()
        if active and now >= next_due:
            emit_latency_event()
            next_due = now + event_interval
        if not ready:
            continue
        raw = sys.stdin.buffer.readline()
        if not raw:
            return 0
        envelope = json.loads(raw)
        message = envelope.get("message")
        if isinstance(message, dict) and not handle(
            message, envelope.get("correlation_id")
        ):
            return 0


if __name__ == "__main__":
    raise SystemExit(main())
