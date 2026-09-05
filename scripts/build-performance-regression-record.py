#!/usr/bin/env python3
"""Build the deterministic, compact M46 performance regression record."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SOURCES = {
    "idle_suite": "artifacts/performance/results/low-overhead/measurement.json",
    "idle_daemon": "artifacts/performance/results/low-overhead/idle-daemon.json",
    "idle_attached": "artifacts/performance/results/low-overhead/idle-attached-client.json",
    "input_latency": "artifacts/performance/input-latency/host-audit.json",
    "ipc_latency": "artifacts/performance/ipc-latency/host-audit.json",
    "ipc_continuity": "artifacts/performance/ipc-gate/manifest.json",
    "memory_endurance": "artifacts/performance/memory/endurance-30m.json",
    "real_poky": "artifacts/performance/real-poky/linux-yocto-do-compile.json",
}


def load(relative: str) -> dict[str, Any]:
    return json.loads((ROOT / relative).read_text(encoding="utf-8"))


def digest(relative: str) -> str:
    return hashlib.sha256((ROOT / relative).read_bytes()).hexdigest()


def metric(value: float | int, unit: str, limit: float | int | None = None) -> dict[str, Any]:
    result: dict[str, Any] = {"value": value, "unit": unit}
    if limit is not None:
        result.update({"hard_maximum": limit, "passed": value <= limit})
    return result


def build_record() -> dict[str, Any]:
    idle_suite = load(SOURCES["idle_suite"])
    idle_daemon = load(SOURCES["idle_daemon"])
    idle_attached = load(SOURCES["idle_attached"])
    input_latency = load(SOURCES["input_latency"])
    ipc_latency = load(SOURCES["ipc_latency"])
    ipc_continuity = load(SOURCES["ipc_continuity"])
    memory = load(SOURCES["memory_endurance"])
    real = load(SOURCES["real_poky"])

    idle_daemon_cpu = idle_daemon["summary"]["processes"]["daemon"]
    idle = idle_attached["summary"]
    real_summary = real["summary"]
    real_response = real["responsiveness"]
    input_summary = input_latency["summary"]
    ipc_summary = ipc_latency["summary"]
    memory_summary = memory["summary"]
    ipc_pressure = ipc_continuity["observations"]

    record = {
        "schema": "yoctui.performance.regression-record.v1",
        "policy": {
            "release_goal": "daemon + one client <=1% of one logical CPU",
            "hard_gates": "controlled release and correctness metrics fail at their documented limits",
            "diagnostics": "informational trends do not fail for tiny uncontrolled variance",
            "comparison": "compare exact values only after matching scenario and method identity",
        },
        "source_artifacts": {
            name: {"path": path, "sha256": digest(path)} for name, path in SOURCES.items()
        },
        "method_identity": {
            "idle": idle_suite["method"],
            "real_poky": real["measurement"],
            "input_latency": input_latency["configuration"],
            "ipc_latency": ipc_latency["configuration"],
            "memory": memory["configuration"],
        },
        "metrics": {
            "cpu": {
                "idle_daemon": metric(
                    idle_daemon_cpu["cpu_trimmed_mean_percent_one_logical_cpu"], "% one logical CPU", 0.2
                ),
                "idle_client": metric(
                    idle["processes"]["client"]["cpu_trimmed_mean_percent_one_logical_cpu"],
                    "% one logical CPU",
                    0.5,
                ),
                "idle_combined": metric(
                    idle["combined_cpu_trimmed_mean_percent_one_logical_cpu"], "% one logical CPU", 1.0
                ),
                "real_poky_daemon": metric(
                    real_summary["daemon"]["cpu_trimmed_mean_percent_one_logical_cpu"],
                    "% one logical CPU",
                ),
                "real_poky_client": metric(
                    real_summary["client"]["cpu_trimmed_mean_percent_one_logical_cpu"],
                    "% one logical CPU",
                ),
                "real_poky_combined": metric(
                    real_summary["combined_cpu_trimmed_mean_percent_one_logical_cpu"],
                    "% one logical CPU",
                    1.0,
                ),
                "real_poky_host": metric(
                    real_summary["host_cpu_trimmed_mean_percent_total_capacity"], "% total host capacity"
                ),
                "real_poky_bitbake": metric(
                    real_summary["bitbake_cpu_trimmed_mean_percent_one_logical_cpu"],
                    "% one logical CPU",
                ),
            },
            "latency": {
                "key_to_model_p95": metric(input_summary["keyboard_to_model_ms"]["p95"], "ms", 100),
                "key_to_frame_p95": metric(input_summary["keyboard_to_visible_frame_ms"]["p95"], "ms", 100),
                "mouse_to_frame_p95": metric(input_summary["mouse_to_visible_selection_ms"]["p95"], "ms", 100),
                "daemon_event_to_client_p95": metric(ipc_summary["daemon_event_to_client_ms"]["p95"], "ms", 100),
                "client_command_to_daemon_p95": metric(ipc_summary["client_command_to_daemon_ms"]["p95"], "ms", 100),
                "cancellation_to_ack_p95": metric(ipc_summary["cancellation_request_to_ack_ms"]["p95"], "ms", 100),
                "real_poky_key_to_frame_p95": metric(real_response["key_to_visible_frame_p95_ms"], "ms", 100),
                "real_poky_cancel_to_ack": metric(real_response["ipc_ms"]["cancellation_command_to_ack_ms"], "ms", 100),
            },
            "wakeups": {
                "idle_daemon_voluntary_context_switches_per_second": metric(
                    idle_daemon_cpu["voluntary_context_switches_mean_per_second"], "context switches/s"
                ),
                "idle_attached_daemon_voluntary_context_switches_per_second": metric(
                    idle["processes"]["daemon"]["voluntary_context_switches_mean_per_second"],
                    "context switches/s",
                ),
                "idle_client_voluntary_context_switches_per_second": metric(
                    idle["processes"]["client"]["voluntary_context_switches_mean_per_second"],
                    "context switches/s",
                ),
            },
            "render": {
                "real_poky_frames_per_second": metric(real["rendering"]["frames_per_second"], "frames/s", 10),
                "real_poky_frames": metric(real["rendering"]["frames"], "frames"),
                "real_poky_coalesced_requests": metric(real["rendering"]["coalesced"], "requests"),
            },
            "pressure": {
                "event_flood_maximum_queue_depth": metric(ipc_pressure["maximum_queue_depth"], "events", 4096),
                "event_flood_forced_resynchronizations": metric(ipc_pressure["forced_resynchronizations"], "count", 0),
                "real_poky_maximum_queue_depth": metric(real["pressure"]["maximum_queue_depth"], "events", 256),
                "real_poky_forced_resynchronizations": metric(real["pressure"]["forced_resynchronizations"], "count", 0),
                "real_poky_backend_disconnects": metric(real["continuity"]["backend_disconnects"], "count", 0),
            },
            "memory": {
                "daemon_rss_growth": metric(memory_summary["daemon"]["rss_growth_bytes"], "bytes", 32 * 1024 * 1024),
                "client_rss_growth": metric(memory_summary["client"]["rss_growth_bytes"], "bytes", 32 * 1024 * 1024),
                "daemon_final_window_slope": metric(
                    memory_summary["daemon"]["final_window_slope_bytes_per_minute"], "bytes/min", 64 * 1024
                ),
                "client_final_window_slope": metric(
                    memory_summary["client"]["final_window_slope_bytes_per_minute"], "bytes/min", 64 * 1024
                ),
                "daemon_rss_max": metric(memory_summary["daemon"]["rss_max_bytes"], "bytes"),
                "client_rss_max": metric(memory_summary["client"]["rss_max_bytes"], "bytes"),
            },
        },
        "correctness": {
            "real_poky_backend_continuity": real["continuity"]["backend_disconnects"] == 0,
            "real_poky_reconnect": real["continuity"]["client_reconnected"],
            "real_poky_cancellation": all(real["continuity"]["cancellation"].values()),
            "memory_critical_retention": memory["retention"]["critical_retention_passed"],
            "memory_strict_order": memory["retention"]["strict_event_order"],
            "memory_connection_continuity": memory["retention"]["connection_continuity"],
            "event_flood_reconnect": ipc_pressure["reconnect_succeeded"],
        },
    }
    hard_metrics = [
        value
        for group in record["metrics"].values()
        for value in group.values()
        if "passed" in value
    ]
    record["result"] = {
        "hard_metrics": len(hard_metrics),
        "hard_metrics_passed": sum(item["passed"] for item in hard_metrics),
        "correctness_checks": len(record["correctness"]),
        "correctness_checks_passed": sum(record["correctness"].values()),
        "passed": all(item["passed"] for item in hard_metrics) and all(record["correctness"].values()),
    }
    return record


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    payload = json.dumps(build_record(), indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(payload, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
