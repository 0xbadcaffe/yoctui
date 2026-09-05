#!/usr/bin/env python3
"""Measure monotonic wake latency for a low-duty interactive process."""

from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import time


SCHEMA = "yoctui.performance.scheduler-latency.v1"


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        raise ValueError("percentile requires a sample")
    ordered = sorted(values)
    index = max(0, math.ceil(fraction * len(ordered)) - 1)
    return ordered[index]


def current_cgroup() -> tuple[str | None, int | None]:
    try:
        unified = next(
            line.split(":", 2)[2]
            for line in Path("/proc/self/cgroup").read_text(encoding="utf-8").splitlines()
            if line.startswith("0::")
        )
    except (OSError, StopIteration):
        return None, None
    try:
        weight = int(Path("/sys/fs/cgroup", unified.lstrip("/"), "cpu.weight").read_text())
    except (OSError, ValueError):
        weight = None
    return unified, weight


def measure(duration: float, interval: float) -> dict[str, object]:
    started = time.monotonic()
    process_started = time.process_time()
    deadline = started + interval
    end = started + duration
    latencies = []
    while deadline <= end:
        remaining = deadline - time.monotonic()
        if remaining > 0:
            time.sleep(remaining)
        observed = time.monotonic()
        latencies.append(max(0.0, (observed - deadline) * 1_000.0))
        deadline += interval
    elapsed = time.monotonic() - started
    cgroup, weight = current_cgroup()
    return {
        "schema": SCHEMA,
        "clock": "CLOCK_MONOTONIC",
        "configuration": {
            "duration_seconds": duration,
            "interval_ms": interval * 1_000.0,
        },
        "process": {
            "nice": os.getpriority(os.PRIO_PROCESS, 0),
            "cgroup": cgroup,
            "cpu_weight": weight,
            "cpu_seconds": time.process_time() - process_started,
        },
        "measurement": {
            "elapsed_seconds": elapsed,
            "samples": len(latencies),
            "wake_latency_ms": {
                "p50": percentile(latencies, 0.50),
                "p95": percentile(latencies, 0.95),
                "p99": percentile(latencies, 0.99),
                "maximum": max(latencies),
            },
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--duration-seconds", type=float, default=2.0)
    parser.add_argument("--interval-ms", type=float, default=10.0)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.duration_seconds < 0.25:
        parser.error("duration must be at least 0.25 seconds")
    if args.interval_ms < 1.0 or args.interval_ms > 1_000.0:
        parser.error("interval must be within 1..1000 milliseconds")
    record = measure(args.duration_seconds, args.interval_ms / 1_000.0)
    rendered = json.dumps(record, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
