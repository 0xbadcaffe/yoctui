#!/usr/bin/env python3
"""Measure bounded daemon/client RSS during a sustained production-path flood."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "yoctui.performance.bounded-memory.v1"
MIB = 1024 * 1024


def process_sample(pid: int) -> dict[str, int]:
    status: dict[str, str] = {}
    for line in Path(f"/proc/{pid}/status").read_text(encoding="utf-8").splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            status[key] = value.strip()
    return {
        "rss_bytes": int(status["VmRSS"].split()[0]) * 1024,
        "threads": int(status["Threads"]),
    }


def direct_children(pid: int) -> list[int]:
    path = Path(f"/proc/{pid}/task/{pid}/children")
    try:
        return [int(value) for value in path.read_text(encoding="utf-8").split()]
    except FileNotFoundError:
        return []


def daemon_child(pid: int) -> int | None:
    for child in direct_children(pid):
        try:
            command = Path(f"/proc/{child}/cmdline").read_bytes().replace(b"\0", b" ")
        except FileNotFoundError:
            continue
        if b"daemon foreground" in command:
            return child
    return None


def least_squares_slope_bytes_per_minute(samples: list[dict[str, object]], role: str) -> float:
    if len(samples) < 2:
        return 0.0
    xs = [float(sample["elapsed_seconds"]) / 60.0 for sample in samples]
    ys = [float(sample["processes"][role]["rss_bytes"]) for sample in samples]
    x_mean = sum(xs) / len(xs)
    y_mean = sum(ys) / len(ys)
    denominator = sum((value - x_mean) ** 2 for value in xs)
    if denominator == 0:
        return 0.0
    return sum((x - x_mean) * (y - y_mean) for x, y in zip(xs, ys)) / denominator


def summarize(samples: list[dict[str, object]], role: str, slope_minutes: int) -> dict[str, object]:
    initial = int(samples[0]["processes"][role]["rss_bytes"])
    rss = [int(sample["processes"][role]["rss_bytes"]) for sample in samples]
    threads = [int(sample["processes"][role]["threads"]) for sample in samples]
    final_start = max(0.0, float(samples[-1]["elapsed_seconds"]) - slope_minutes * 60)
    final_samples = [sample for sample in samples if float(sample["elapsed_seconds"]) >= final_start]
    return {
        "rss_initial_bytes": initial,
        "rss_final_bytes": rss[-1],
        "rss_max_bytes": max(rss),
        "rss_growth_bytes": max(rss) - initial,
        "final_window_minutes": slope_minutes,
        "final_window_slope_bytes_per_minute": least_squares_slope_bytes_per_minute(
            final_samples, role
        ),
        "threads_initial": threads[0],
        "threads_final": threads[-1],
        "threads_max": max(threads),
    }


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--warmup-seconds", type=int, default=10)
    parser.add_argument("--sample-seconds", type=int, default=60)
    parser.add_argument("--rate", type=int, default=4_000)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.warmup_seconds < 1 or args.sample_seconds < 30 or args.rate < 2_000:
        parser.error("warmup must be >=1, samples >=30, and rate >=2000")
    binary = args.binary.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="yoctui-memory-") as directory:
        flood_output = Path(directory) / "flood.json"
        command = [
            sys.executable,
            str(ROOT / "scripts/event-flood-harness.py"),
            "--binary", str(binary),
            "--rate", str(args.rate),
            "--duration-seconds", str(args.warmup_seconds + args.sample_seconds),
            "--observation-seconds", "1.5",
            "--include-slow-client",
            "--output", str(flood_output),
        ]
        child = subprocess.Popen(
            command, cwd=ROOT, stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True,
        )
        started = time.monotonic()
        daemon_pid = None
        deadline = started + 20
        while time.monotonic() < deadline and child.poll() is None:
            daemon_pid = daemon_child(child.pid)
            if daemon_pid is not None:
                break
            time.sleep(0.02)
        if daemon_pid is None:
            child.kill()
            raise RuntimeError("memory harness did not observe the daemon child")
        samples: list[dict[str, object]] = []
        next_sample = started + args.warmup_seconds
        end_sample = next_sample + args.sample_seconds
        while time.monotonic() <= end_sample:
            if child.poll() is not None:
                stderr = child.stderr.read() if child.stderr else ""
                raise RuntimeError(f"event flood ended before memory window: {stderr}")
            now = time.monotonic()
            if now >= next_sample:
                try:
                    process_values = {
                        "daemon": process_sample(daemon_pid),
                        "client": process_sample(child.pid),
                    }
                except FileNotFoundError as error:
                    child.kill()
                    child.wait()
                    stderr = child.stderr.read() if child.stderr else ""
                    raise RuntimeError(
                        f"measured process exited during memory window: {stderr}"
                    ) from error
                samples.append({
                    "elapsed_seconds": now - started,
                    "processes": process_values,
                })
                next_sample += 1.0
            time.sleep(min(0.05, max(0.0, next_sample - time.monotonic())))
        try:
            child.wait(timeout=15)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait()
            raise RuntimeError("event flood did not terminate after the measurement")
        if child.returncode != 0:
            stderr = child.stderr.read() if child.stderr else ""
            raise RuntimeError(f"event flood failed: {stderr}")
        flood = json.loads(flood_output.read_text(encoding="utf-8"))
    if len(samples) < args.sample_seconds - 1:
        raise RuntimeError("memory sample window is incomplete")
    endurance = args.sample_seconds >= 1_800
    slope_minutes = 20 if endurance else max(1, min(20, args.sample_seconds // 120))
    summaries = {role: summarize(samples, role, slope_minutes) for role in ("daemon", "client")}
    for role, summary in summaries.items():
        if int(summary["rss_growth_bytes"]) > 32 * MIB:
            raise RuntimeError(f"{role} RSS grew beyond 32 MiB")
        if int(summary["threads_max"]) > int(summary["threads_initial"]):
            raise RuntimeError(f"{role} thread count grew after warmup")
        if endurance and float(summary["final_window_slope_bytes_per_minute"]) > 64 * 1024:
            raise RuntimeError(f"{role} final RSS slope exceeds 64 KiB/min")
    record = {
        "schema": SCHEMA,
        "source_base_revision": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip(),
        "binary": {"path": str(binary), "sha256": sha256(binary)},
        "configuration": {
            "clock": "CLOCK_MONOTONIC",
            "warmup_seconds": args.warmup_seconds,
            "sample_window_seconds": args.sample_seconds,
            "sample_interval_seconds": 1,
            "event_rate_per_second": args.rate,
            "production_path": flood["configuration"]["production_path"],
            "attached_client": "bounded protocol observer",
            "endurance_release_evidence": endurance,
        },
        "summary": summaries,
        "retention": {
            "daemon": flood["bounds"],
            "pressure": flood["client"]["pressure"],
            "critical_retention_passed": flood["result"]["critical_retention_passed"],
            "strict_event_order": flood["client"]["event_sequences_strictly_increasing"],
            "connection_continuity": flood["client"]["connection_continuity"],
        },
        "samples": samples,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summaries, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
