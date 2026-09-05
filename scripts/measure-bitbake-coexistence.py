#!/usr/bin/env python3
"""Measure scheduler coexistence at normal and explicit CPU oversubscription."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile
import time


SCHEMA = "yoctui.performance.bitbake-coexistence.v1"


def wait_ready(process: subprocess.Popen[str], event_log: Path) -> None:
    deadline = time.monotonic() + 10.0
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
    worker_count: int,
    duration: float,
) -> tuple[dict[str, object], dict[str, object]]:
    saturation = directory / "saturation.json"
    events = directory / "saturation.jsonl"
    load = subprocess.Popen(
        [
            str(root / "scripts/cpu-saturation-harness.py"),
            "--workers",
            str(worker_count),
            "--warmup-seconds",
            "0.25",
            "--duration-seconds",
            str(duration + 1.0),
            "--minimum-worker-cpu-percent",
            "15",
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
        probe = subprocess.run(
            [
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
        if probe.returncode != 0:
            raise RuntimeError(f"scheduler probe failed: {probe.stderr.strip()}")
        return json.loads(probe.stdout), json.loads(saturation.read_text(encoding="utf-8"))
    finally:
        if load.poll() is None:
            load.terminate()
            try:
                load.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                load.kill()
                load.wait(timeout=1.0)


def scenario_summary(trials: list[dict[str, object]]) -> dict[str, float]:
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
    configuration = record["configuration"]
    achieved = record["achieved"]
    return {
        "status": record["status"],
        "requested_workers": configuration["requested_workers"],
        "selected_cpus": configuration["selected_cpus"],
        "worker_cpu_assignments": configuration["worker_cpu_assignments"],
        "minimum_worker_cpu_percent": achieved["minimum_worker_cpu_percent"],
        "mean_worker_cpu_percent": achieved["mean_worker_cpu_percent"],
        "host_cpu_utilization_percent": achieved["host_cpu_utilization_percent"],
        "load_average_before": achieved["load_average_before"],
        "load_average_after": achieved["load_average_after"],
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

    affinity = sorted(os.sched_getaffinity(0))
    root = Path(__file__).resolve().parents[1]
    fixture_root = Path(tempfile.mkdtemp(prefix="yoctui-coexistence-audit-"))
    scenarios = {}
    try:
        for name, workers in (
            ("one_worker_per_logical_cpu", len(affinity)),
            ("two_workers_per_logical_cpu", len(affinity) * 2),
        ):
            trials = []
            loads = []
            for repetition in range(args.repetitions):
                directory = fixture_root / f"{name}-{repetition}"
                directory.mkdir()
                trial, load = run_trial(root, directory, workers, args.duration_seconds)
                trials.append(trial)
                loads.append(load_summary(load))
            scenarios[name] = {
                "workers": workers,
                "trials": trials,
                "saturation": loads,
                "summary": scenario_summary(trials),
            }

        source_paths = {
            "harness": root / "scripts/cpu-saturation-harness.py",
            "probe": root / "scripts/scheduler-latency-probe.py",
            "measurement": Path(__file__),
            "scheduling_evidence": root / "artifacts/performance/scheduling/manifest.json",
            "affinity_evidence": root / "artifacts/performance/affinity/manifest.json",
        }
        record = {
            "schema": SCHEMA,
            "revision": args.revision,
            "captured_at_unix_ms": int(time.time() * 1_000),
            "host": {
                "logical_cpus": os.cpu_count(),
                "affinity_cpus": affinity,
                "kernel": os.uname().release,
                "cgroup_v2": Path("/sys/fs/cgroup/cgroup.controllers").exists(),
            },
            "configuration": {
                "probe_duration_seconds": args.duration_seconds,
                "probe_interval_ms": 10,
                "repetitions": args.repetitions,
                "review_example_jobs": max(1, len(affinity) - 1),
            },
            "scenarios": scenarios,
            "policy": {
                "diagnostic": "read-only",
                "parallelism": "inspect BB_NUMBER_THREADS and PARALLEL_MAKE independently; do not multiply them",
                "default": "retain user configuration and inherited scheduling, affinity, and cgroup",
                "example": "one fewer job than available logical CPUs is review-only, never automatic",
                "correctness": "requires neither root nor a deliberately reserved CPU",
            },
            "sources": {
                name: {
                    "path": str(path.relative_to(root)),
                    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                }
                for name, path in source_paths.items()
            },
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
