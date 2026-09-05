#!/usr/bin/env python3
"""Deterministic offline BitBake-like bridge used only by performance gates."""

from __future__ import annotations

import json
import os
from pathlib import Path
import selectors
import sys
import time


PROTOCOL_VERSION = 1
MAX_EVENTS_PER_SLICE = 256
SCHEMA = "yoctui.performance.event-flood-generator.v1"
CRITICAL_RECIPE = "perf-critical"
CRITICAL_TASK = "do_failure"
WARNING_SENTINEL = "PERF_CRITICAL_WARNING"
ERROR_SENTINEL = "PERF_CRITICAL_ERROR"

sequence = 0
active = False
build_correlation: str | None = None
started_at = 0.0
emitted_events = 0
event_counts: dict[str, int] = {}
critical_sent: list[dict[str, object]] = []
configured_rate = 0
configured_duration = 0.0
configured_profile = ""
configured_terminal = ""
report_path: Path | None = None


def configuration() -> None:
    global configured_rate, configured_duration, configured_profile
    global configured_terminal, report_path
    try:
        configured_rate = int(os.environ.get("YOCTUI_PERF_EVENT_RATE", "2000"))
        configured_duration = float(os.environ.get("YOCTUI_PERF_EVENT_DURATION", "90"))
    except ValueError as error:
        raise RuntimeError("event rate and duration must be numeric") from error
    configured_profile = os.environ.get("YOCTUI_PERF_EVENT_PROFILE", "balanced")
    configured_terminal = os.environ.get("YOCTUI_PERF_EVENT_TERMINAL", "success")
    report = os.environ.get("YOCTUI_PERF_EVENT_REPORT")
    report_path = Path(report) if report else None
    if configured_rate <= 0 or configured_duration < 0.1:
        raise RuntimeError(
            "event rate must be positive and duration at least 0.1 seconds"
        )
    if configured_profile not in {"balanced", "log-heavy", "task-heavy"}:
        raise RuntimeError(f"unknown YOCTUI_PERF_EVENT_PROFILE {configured_profile!r}")
    if configured_terminal not in {"success", "failure", "disconnect"}:
        raise RuntimeError(
            f"unknown YOCTUI_PERF_EVENT_TERMINAL {configured_terminal!r}"
        )


def emit(message: dict[str, object], correlation: str | None = None) -> int:
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
    return sequence


def emit_build(message: dict[str, object], critical: str | None = None) -> None:
    kind = str(message["type"])
    event_counts[kind] = event_counts.get(kind, 0) + 1
    emitted_sequence = emit(message, build_correlation)
    if critical is not None:
        critical_sent.append(
            {"name": critical, "bridge_sequence": emitted_sequence, "type": kind}
        )


def event_for(index: int) -> dict[str, object]:
    if configured_profile == "log-heavy":
        task_index = index // 64
        recipe = f"fixture-recipe-{task_index % 128:03d}"
        task = f"do_fixture_{task_index % 16:02d}"
        if index % 64 == 0:
            return {"type": "warning", "message": f"fixture warning {index}"}
        if index % 64 == 1:
            return {"type": "error", "message": f"fixture error {index}"}
        return {
            "type": "log",
            "level": "info",
            "message": f"fixture ordinary log {index}",
            "recipe": recipe,
            "task": task,
            "path": None,
        }
    if configured_profile == "task-heavy":
        task_index = index // 4
        recipe = f"fixture-recipe-{task_index % 128:03d}"
        task = f"do_fixture_{task_index % 16:02d}"
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
            "success": task_index % 31 != 30,
        }

    task_index = index // 32
    recipe = f"fixture-recipe-{task_index % 128:03d}"
    task = f"do_fixture_{task_index % 16:02d}"
    kind = index % 32
    if kind == 0:
        return {"type": "task_queued", "recipe": recipe, "task": task}
    if kind == 1:
        return {
            "type": "task_started",
            "recipe": recipe,
            "task": task,
            "pid": 10_000 + task_index % 1_000,
        }
    if 2 <= kind <= 5:
        return {
            "type": "task_progress",
            "recipe": recipe,
            "task": task,
            "progress": min(99, (kind - 1) * 20),
        }
    if kind == 29:
        return {"type": "warning", "message": f"fixture warning {index}"}
    if kind == 30:
        return {"type": "error", "message": f"fixture error {index}"}
    if kind == 31:
        return {
            "type": "task_completed",
            "recipe": recipe,
            "task": task,
            "success": task_index % 31 != 30,
        }
    return {
        "type": "log",
        "level": "info",
        "message": f"fixture ordinary log {index}",
        "recipe": recipe,
        "task": task,
        "path": None,
    }


def write_report(outcome: str) -> None:
    elapsed = max(time.monotonic() - started_at, 0.000_001)
    record = {
        "schema": SCHEMA,
        "outcome": outcome,
        "configuration": {
            "rate_events_per_second": configured_rate,
            "duration_seconds": configured_duration,
            "profile": configured_profile,
            "terminal": configured_terminal,
        },
        "measurement": {
            "clock": "CLOCK_MONOTONIC",
            "elapsed_seconds": elapsed,
            "ordinary_events": emitted_events,
            "total_build_events": sum(event_counts.values()),
            "actual_ordinary_events_per_second": emitted_events / elapsed,
            "event_counts": event_counts,
        },
        "critical_sent": critical_sent,
        "last_bridge_sequence": sequence,
    }
    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        temporary = report_path.with_suffix(report_path.suffix + ".tmp")
        temporary.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        temporary.replace(report_path)


def emit_critical_tail() -> bool:
    emit_build({"type": "warning", "message": WARNING_SENTINEL}, "warning_sentinel")
    emit_build({"type": "error", "message": ERROR_SENTINEL}, "error_sentinel")
    identity = {"recipe": CRITICAL_RECIPE, "task": CRITICAL_TASK}
    emit_build({"type": "task_queued", **identity}, "critical_task_queued")
    emit_build(
        {"type": "task_started", **identity, "pid": 99_999},
        "critical_task_started",
    )
    emit_build(
        {"type": "task_progress", **identity, "progress": 99},
        "critical_task_progress",
    )
    emit_build(
        {"type": "task_completed", **identity, "success": False},
        "critical_task_failed",
    )
    if configured_terminal == "disconnect":
        write_report("backend_disconnected")
        return False
    success = configured_terminal == "success"
    emit_build(
        {
            "type": "build_completed",
            "success": success,
            "exit_code": 0 if success else 1,
        },
        "build_terminal",
    )
    write_report("completed" if success else "failed")
    return True


def emit_due_events() -> bool:
    global active, emitted_events
    elapsed = time.monotonic() - started_at
    due = min(int(elapsed * configured_rate) - emitted_events, MAX_EVENTS_PER_SLICE)
    for _ in range(max(0, due)):
        emit_build(event_for(emitted_events))
        emitted_events += 1
    if elapsed >= configured_duration:
        active = False
        return emit_critical_tail()
    return True


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
        event_counts.clear()
        critical_sent.clear()
        started_at = time.monotonic()
        emit_build({"type": "build_started"})
    elif kind == "cancel_build":
        active = False
        emit_build(
            {"type": "error", "message": "PERF_CRITICAL_CANCELLATION"},
            "cancellation",
        )
        emit_build(
            {"type": "build_completed", "success": False, "exit_code": 130},
            "cancellation_terminal",
        )
        write_report("cancelled")
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
    try:
        configuration()
    except RuntimeError as error:
        print(f"event flood configuration error: {error}", file=sys.stderr)
        return 2
    selector = selectors.DefaultSelector()
    selector.register(sys.stdin.buffer, selectors.EVENT_READ)
    while True:
        ready = selector.select(0.001 if active else 1.0)
        if active and not emit_due_events():
            return 0
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
