#!/usr/bin/env python3
"""Compare safe process scheduling options under full-affinity CPU load."""

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


SCHEMA = "yoctui.performance.scheduling-audit.v1"


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


def run_loaded_probe(
    root: Path,
    fixture: Path,
    probe_command: list[str],
    duration: float,
) -> tuple[dict[str, object], dict[str, object]]:
    saturation = fixture / "saturation.json"
    events = fixture / "saturation.jsonl"
    load = subprocess.Popen(
        [
            str(root / "scripts/cpu-saturation-harness.py"),
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
            probe_command,
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
            raise RuntimeError(f"scheduler probe failed: {result.stderr.strip()}")
        return json.loads(result.stdout), json.loads(saturation.read_text(encoding="utf-8"))
    finally:
        if load.poll() is None:
            load.terminate()
            try:
                load.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                load.kill()
                load.wait(timeout=1.0)


def probe_command(probe: Path, duration: float) -> list[str]:
    return [
        str(probe),
        "--duration-seconds",
        str(duration),
        "--interval-ms",
        "10",
    ]


def load_summary(record: dict[str, object]) -> dict[str, object]:
    configuration = record["configuration"]
    achieved = record["achieved"]
    return {
        "status": record["status"],
        "selected_cpus": configuration["selected_cpus"],
        "default_saturates_full_affinity": configuration[
            "default_saturates_full_affinity"
        ],
        "minimum_worker_cpu_percent": achieved["minimum_worker_cpu_percent"],
        "mean_worker_cpu_percent": achieved["mean_worker_cpu_percent"],
        "host_cpu_utilization_percent": achieved["host_cpu_utilization_percent"],
        "children_reaped": record["cleanup"]["children_reaped"],
    }


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

    root = Path(__file__).resolve().parents[1]
    probe = root / "scripts/scheduler-latency-probe.py"
    fixture_root = Path(tempfile.mkdtemp(prefix="yoctui-scheduling-audit-"))
    scenarios = {}
    saturation = {}
    try:
        for name, prefix in (
            ("inherited_nice_0", []),
            ("deprioritized_nice_5", ["nice", "-n", "5"]),
        ):
            trials = []
            loads = []
            for repetition in range(args.repetitions):
                directory = fixture_root / f"{name}-{repetition}"
                directory.mkdir()
                trial, load = run_loaded_probe(
                    root,
                    directory,
                    prefix + probe_command(probe, args.duration_seconds),
                    args.duration_seconds,
                )
                trials.append(trial)
                loads.append(load_summary(load))
            scenarios[name] = {
                "trials": trials,
                "summary": scenario_summary(trials),
            }
            saturation[name] = loads

        systemd_run = shutil.which("systemd-run")
        systemd_available = False
        systemd_error = None
        if systemd_run:
            try:
                trials = []
                loads = []
                for repetition in range(args.repetitions):
                    directory = fixture_root / f"cpu_weight_200-{repetition}"
                    directory.mkdir()
                    command = [
                        systemd_run,
                        "--user",
                        "--quiet",
                        "--pipe",
                        "--wait",
                        "--collect",
                        "-p",
                        "CPUWeight=200",
                        *probe_command(probe, args.duration_seconds),
                    ]
                    trial, load = run_loaded_probe(
                        root, directory, command, args.duration_seconds
                    )
                    trials.append(trial)
                    loads.append(load_summary(load))
                scenarios["cpu_weight_200"] = {
                    "trials": trials,
                    "summary": scenario_summary(trials),
                }
                saturation["cpu_weight_200"] = loads
                systemd_available = True
            except (RuntimeError, subprocess.SubprocessError, json.JSONDecodeError) as error:
                systemd_error = str(error)

        negative = subprocess.run(
            ["nice", "-n", "-5", *probe_command(probe, 0.25)],
            cwd=root,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            timeout=3.0,
            check=False,
        )
        negative_record = json.loads(negative.stdout)
        negative_available = negative_record["process"]["nice"] < 0

        record = {
            "schema": SCHEMA,
            "revision": args.revision,
            "captured_at_unix_ms": int(time.time() * 1_000),
            "host": {
                "logical_cpus": os.cpu_count(),
                "affinity_cpus": len(os.sched_getaffinity(0)),
                "kernel": os.uname().release,
                "cgroup_v2": Path("/sys/fs/cgroup/cgroup.controllers").exists(),
            },
            "configuration": {
                "probe_duration_seconds": args.duration_seconds,
                "probe_interval_ms": 10,
                "repetitions": args.repetitions,
                "load_workers": "all affinity CPUs",
            },
            "capabilities": {
                "negative_nice_unprivileged": negative_available,
                "negative_nice_observed": negative_record["process"]["nice"],
                "negative_nice_stderr": negative.stderr.strip(),
                "systemd_user_cpu_weight": systemd_available,
                "systemd_user_error": systemd_error,
            },
            "scenarios": scenarios,
            "saturation": saturation,
            "source": {
                "probe_sha256": hashlib.sha256(probe.read_bytes()).hexdigest(),
                "harness_sha256": hashlib.sha256(
                    (root / "scripts/cpu-saturation-harness.py").read_bytes()
                ).hexdigest(),
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
