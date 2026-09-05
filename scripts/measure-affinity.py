#!/usr/bin/env python3
"""Compare shared and one-logical-CPU-reserved scheduling under load."""

from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile
import time


SCHEMA = "yoctui.performance.affinity-audit.v1"


def cpu_list(values: list[int]) -> str:
    return ",".join(str(value) for value in values)


def wait_ready(process: subprocess.Popen[str], event_log: Path) -> None:
    deadline = time.monotonic() + 8.0
    while process.poll() is None and time.monotonic() < deadline:
        if event_log.exists() and '"event":"ready"' in event_log.read_text(
            encoding="utf-8"
        ):
            time.sleep(0.3)
            return
        time.sleep(0.02)
    raise RuntimeError("CPU saturation fixture did not become ready")


def run_trial(
    root: Path,
    directory: Path,
    load_cpus: list[int],
    probe_cpus: list[int],
    duration: float,
) -> tuple[dict[str, object], dict[str, object]]:
    saturation = directory / "saturation.json"
    events = directory / "saturation.jsonl"
    load = subprocess.Popen(
        [
            str(root / "scripts/cpu-saturation-harness.py"),
            "--cpu-list",
            cpu_list(load_cpus),
            "--warmup-seconds",
            "0.25",
            "--duration-seconds",
            str(duration + 1.0),
            "--minimum-worker-cpu-percent",
            "25",
            "--event-log",
            str(events),
            "--output",
            str(saturation),
        ],
        cwd=root,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        wait_ready(load, events)
        result = subprocess.run(
            [
                "taskset",
                "--cpu-list",
                cpu_list(probe_cpus),
                str(root / "scripts/scheduler-latency-probe.py"),
                "--duration-seconds",
                str(duration),
                "--interval-ms",
                "10",
            ],
            cwd=root,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            timeout=duration + 5.0,
            check=False,
        )
        load_stderr = load.communicate(timeout=duration + 3.0)[1]
        if load.returncode != 0:
            raise RuntimeError(f"CPU saturation fixture failed: {load_stderr.strip()}")
        if result.returncode != 0:
            raise RuntimeError(f"affinity probe failed: {result.stderr.strip()}")
        probe = json.loads(result.stdout)
        probe["requested_affinity_cpus"] = probe_cpus
        observed = sorted(os.sched_getaffinity(0))
        # The child has exited, so taskset's accepted CPU set is the reproducible
        # authority; host affinity is retained separately for validation.
        probe["parent_affinity_cpus"] = observed
        return probe, json.loads(saturation.read_text(encoding="utf-8"))
    finally:
        if load.poll() is None:
            load.terminate()
            try:
                load.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                load.kill()
                load.wait(timeout=1.0)


def percentile_summary(trials: list[dict[str, object]]) -> dict[str, float]:
    p95 = [trial["measurement"]["wake_latency_ms"]["p95"] for trial in trials]
    maximum = [
        trial["measurement"]["wake_latency_ms"]["maximum"] for trial in trials
    ]
    return {
        "median_p95_wake_latency_ms": statistics.median(p95),
        "worst_p95_wake_latency_ms": max(p95),
        "worst_maximum_wake_latency_ms": max(maximum),
    }


def load_summary(record: dict[str, object]) -> dict[str, object]:
    return {
        "status": record["status"],
        "selected_cpus": record["configuration"]["selected_cpus"],
        "minimum_worker_cpu_percent": record["achieved"][
            "minimum_worker_cpu_percent"
        ],
        "host_cpu_utilization_percent": record["achieved"][
            "host_cpu_utilization_percent"
        ],
        "children_reaped": record["cleanup"]["children_reaped"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--revision", required=True)
    parser.add_argument("--duration-seconds", type=float, default=3.0)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if len(args.revision) != 40:
        parser.error("revision must be a full 40-character Git commit")
    if args.duration_seconds < 1.0:
        parser.error("duration must be at least one second")
    if args.repetitions < 1 or args.repetitions > 10:
        parser.error("repetitions must be within 1..10")
    allowed = sorted(os.sched_getaffinity(0))
    if len(allowed) < 2:
        parser.error("affinity comparison requires at least two logical CPUs")

    root = Path(__file__).resolve().parents[1]
    fixture_root = Path(tempfile.mkdtemp(prefix="yoctui-affinity-audit-"))
    reserved = allowed[-1]
    shared_load = allowed
    reserved_load = allowed[:-1]
    scenarios = {
        "shared_full_affinity": (shared_load, allowed),
        "pinned_competing_cpu": (shared_load, [reserved]),
        "pinned_reserved_cpu": (reserved_load, [reserved]),
    }
    results = {}
    try:
        for name, (load_cpus, probe_cpus) in scenarios.items():
            trials = []
            loads = []
            for repetition in range(args.repetitions):
                directory = fixture_root / f"{name}-{repetition}"
                directory.mkdir()
                probe, load = run_trial(
                    root, directory, load_cpus, probe_cpus, args.duration_seconds
                )
                trials.append(probe)
                loads.append(load_summary(load))
            results[name] = {
                "load_cpus": load_cpus,
                "probe_cpus": probe_cpus,
                "trials": trials,
                "saturation": loads,
                "summary": percentile_summary(trials),
            }

        sibling_path = Path(
            f"/sys/devices/system/cpu/cpu{reserved}/topology/thread_siblings_list"
        )
        record = {
            "schema": SCHEMA,
            "revision": args.revision,
            "captured_at_unix_ms": math.floor(time.time() * 1_000),
            "host": {
                "affinity_cpus": allowed,
                "logical_cpus": os.cpu_count(),
                "reserved_logical_cpu": reserved,
                "reserved_cpu_thread_siblings": sibling_path.read_text().strip()
                if sibling_path.exists()
                else None,
            },
            "configuration": {
                "probe_duration_seconds": args.duration_seconds,
                "probe_interval_ms": 10,
                "repetitions": args.repetitions,
            },
            "scenarios": results,
        }
        rendered = json.dumps(record, indent=2, sort_keys=True) + "\n"
        args.output.parent.mkdir(parents=True, exist_ok=True)
        temporary = args.output.with_suffix(args.output.suffix + ".tmp")
        temporary.write_text(rendered, encoding="utf-8")
        temporary.replace(args.output)
        print(rendered, end="")
        return 0
    finally:
        shutil.rmtree(fixture_root)


if __name__ == "__main__":
    raise SystemExit(main())
