#!/usr/bin/env python3
"""Measure steady-state Linux process overhead using the M46 contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import statistics
import subprocess
import time
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path


SCHEMA = "yoctui.performance.process-overhead.v1"


@dataclass(frozen=True)
class ProcessSample:
    ticks: int
    voluntary_switches: int
    involuntary_switches: int
    rss_bytes: int
    virtual_bytes: int
    threads: int


def parse_pid(value: str) -> tuple[str, int]:
    try:
        name, raw_pid = value.split("=", 1)
        pid = int(raw_pid)
    except (ValueError, TypeError) as error:
        raise argparse.ArgumentTypeError("PID must be ROLE=NUMBER") from error
    if not name or pid <= 0:
        raise argparse.ArgumentTypeError("PID must be ROLE=NUMBER")
    return name, pid


def proc_sample(pid: int) -> ProcessSample:
    stat = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8")
    # The command name is parenthesized and may contain spaces. Fields after the
    # final ')' begin at documented proc_pid_stat field 3.
    fields = stat[stat.rfind(")") + 2 :].split()
    ticks = int(fields[11]) + int(fields[12])
    status: dict[str, str] = {}
    for line in Path(f"/proc/{pid}/status").read_text(encoding="utf-8").splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            status[key] = value.strip()

    def kib(name: str) -> int:
        raw = status.get(name, "0 kB").split()[0]
        return int(raw) * 1024

    return ProcessSample(
        ticks=ticks,
        voluntary_switches=int(status["voluntary_ctxt_switches"]),
        involuntary_switches=int(status["nonvoluntary_ctxt_switches"]),
        rss_bytes=kib("VmRSS"),
        virtual_bytes=kib("VmSize"),
        threads=int(status["Threads"]),
    )


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def trimmed_mean(values: list[float]) -> float:
    if not values:
        raise ValueError("no samples")
    ordered = sorted(values)
    trim = len(ordered) // 10
    retained = ordered[trim : len(ordered) - trim] if trim else ordered
    return statistics.fmean(retained)


def git_output(*arguments: str) -> str:
    return subprocess.check_output(
        ["git", *arguments], text=True, stderr=subprocess.DEVNULL
    ).strip()


def command_output(*arguments: str) -> str:
    return subprocess.check_output(arguments, text=True, stderr=subprocess.DEVNULL).strip()


def cpu_model() -> str:
    for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines():
        if line.startswith("model name"):
            return line.split(":", 1)[1].strip()
    return "unknown"


def memory_total_bytes() -> int:
    for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("MemTotal is absent from /proc/meminfo")


def scaling_governors() -> list[str]:
    return sorted(
        {
            path.read_text(encoding="utf-8").strip()
            for path in Path("/sys/devices/system/cpu").glob(
                "cpu[0-9]*/cpufreq/scaling_governor"
            )
        }
    )


def process_identity(pid: int) -> dict[str, object]:
    stat = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8")
    fields = stat[stat.rfind(")") + 2 :].split()
    return {
        "pid": pid,
        "start_time_ticks_since_boot": int(fields[19]),
        "executable": str(Path(f"/proc/{pid}/exe").resolve()),
        "command": Path(f"/proc/{pid}/cmdline")
        .read_bytes()
        .rstrip(b"\0")
        .replace(b"\0", b" ")
        .decode("utf-8", errors="replace"),
    }


def parse_metric(value: str) -> tuple[str, object]:
    try:
        name, raw_value = value.split("=", 1)
    except ValueError as error:
        raise argparse.ArgumentTypeError("metric must be NAME=VALUE") from error
    if not name:
        raise argparse.ArgumentTypeError("metric name must not be empty")
    try:
        parsed: object = float(raw_value)
    except ValueError:
        parsed = raw_value
    return name, parsed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--pid", action="append", required=True, type=parse_pid)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--warmup-seconds", type=int, default=10)
    parser.add_argument("--sample-seconds", type=int, default=60)
    parser.add_argument("--terminal-columns", type=int, default=160)
    parser.add_argument("--terminal-rows", type=int, default=50)
    parser.add_argument("--refresh-milliseconds", type=int, default=100)
    parser.add_argument("--filesystem-path", type=Path, default=Path.cwd())
    parser.add_argument("--observed-metric", action="append", default=[], type=parse_metric)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.warmup_seconds < 0 or args.sample_seconds < 2:
        parser.error("warmup must be nonnegative and sample window must be at least 2 seconds")

    processes = dict(args.pid)
    if len(processes) != len(args.pid):
        parser.error("process role names must be unique")
    binary = args.binary.resolve(strict=True)
    filesystem_path = args.filesystem_path.resolve(strict=True)
    initial_identities = {role: process_identity(pid) for role, pid in processes.items()}
    for role, pid in processes.items():
        if not Path(f"/proc/{pid}").is_dir():
            parser.error(f"{role} PID {pid} is not running")

    load_average_before = list(os.getloadavg())
    time.sleep(args.warmup_seconds)
    clock_ticks = os.sysconf("SC_CLK_TCK")
    previous_time = time.monotonic()
    previous = {role: proc_sample(pid) for role, pid in processes.items()}
    samples: list[dict[str, object]] = []
    deadline = previous_time + args.sample_seconds
    while previous_time < deadline:
        time.sleep(min(1.0, max(0.0, deadline - time.monotonic())))
        now = time.monotonic()
        elapsed = now - previous_time
        current = {role: proc_sample(pid) for role, pid in processes.items()}
        process_values: dict[str, object] = {}
        combined_cpu = 0.0
        for role in processes:
            old = previous[role]
            new = current[role]
            cpu = (new.ticks - old.ticks) / clock_ticks / elapsed * 100.0
            combined_cpu += cpu
            process_values[role] = {
                "cpu_percent_one_logical_cpu": cpu,
                "voluntary_context_switches_per_second": (
                    new.voluntary_switches - old.voluntary_switches
                )
                / elapsed,
                "involuntary_context_switches_per_second": (
                    new.involuntary_switches - old.involuntary_switches
                )
                / elapsed,
                "rss_bytes": new.rss_bytes,
                "virtual_bytes": new.virtual_bytes,
                "threads": new.threads,
            }
        samples.append(
            {
                "elapsed_seconds": elapsed,
                "combined_cpu_percent_one_logical_cpu": combined_cpu,
                "processes": process_values,
            }
        )
        previous_time = now
        previous = current

    summaries: dict[str, object] = {}
    for role in processes:
        role_samples = [sample["processes"][role] for sample in samples]
        cpu_values = [item["cpu_percent_one_logical_cpu"] for item in role_samples]
        summaries[role] = {
            "cpu_trimmed_mean_percent_one_logical_cpu": trimmed_mean(cpu_values),
            "cpu_median_percent_one_logical_cpu": statistics.median(cpu_values),
            "voluntary_context_switches_mean_per_second": statistics.fmean(
                item["voluntary_context_switches_per_second"] for item in role_samples
            ),
            "involuntary_context_switches_mean_per_second": statistics.fmean(
                item["involuntary_context_switches_per_second"] for item in role_samples
            ),
            "rss_max_bytes": max(item["rss_bytes"] for item in role_samples),
            "virtual_max_bytes": max(item["virtual_bytes"] for item in role_samples),
            "threads_max": max(item["threads"] for item in role_samples),
        }
    combined = [sample["combined_cpu_percent_one_logical_cpu"] for sample in samples]
    final_identities = {role: process_identity(pid) for role, pid in processes.items()}
    if final_identities != initial_identities:
        raise RuntimeError("a measured process changed identity during the sample window")
    filesystem = os.statvfs(filesystem_path)
    filesystem_type = command_output("findmnt", "-n", "-o", "FSTYPE", "-T", str(filesystem_path))
    record = {
        "schema": SCHEMA,
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
        "scenario": args.scenario,
        "measurement": {
            "clock": "CLOCK_MONOTONIC",
            "cpu_source": "/proc/PID/stat fields 14+15",
            "clock_ticks_per_second": clock_ticks,
            "warmup_seconds": args.warmup_seconds,
            "sample_window_seconds": args.sample_seconds,
            "sample_count": len(samples),
            "statistic": "10_percent_trimmed_mean",
        },
        "host": {
            "kernel": platform.release(),
            "machine": platform.machine(),
            "cpu_model": cpu_model(),
            "logical_cpus": os.cpu_count(),
            "online_cpus": Path("/sys/devices/system/cpu/online").read_text().strip(),
            "memory_total_bytes": memory_total_bytes(),
            "scaling_governors": scaling_governors(),
            "load_average_before": load_average_before,
            "load_average_after": list(os.getloadavg()),
            "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "filesystem": {
                "path": str(filesystem_path),
                "type": filesystem_type,
                "free_bytes": filesystem.f_bavail * filesystem.f_frsize,
                "total_bytes": filesystem.f_blocks * filesystem.f_frsize,
            },
        },
        "terminal": {
            "columns": args.terminal_columns,
            "rows": args.terminal_rows,
            "refresh_milliseconds": args.refresh_milliseconds,
        },
        "revision": git_output("rev-parse", "HEAD"),
        "worktree_clean": not bool(git_output("status", "--porcelain")),
        "rustc": command_output("rustc", "--version"),
        "binary": {
            "path": str(binary),
            "sha256": sha256(binary),
            "version_output": command_output(str(binary), "--version"),
        },
        "processes": initial_identities,
        "supplemental_metrics": dict(args.observed_metric),
        "measurement_availability": {
            "scheduler_context_switches": True,
            "process_wakeups": False,
            "process_wakeups_reason": (
                "Linux /proc does not expose per-process wakeups; perf scheduling "
                "samples are captured by the profiling task"
            ),
        },
        "summary": {
            "combined_cpu_trimmed_mean_percent_one_logical_cpu": trimmed_mean(combined),
            "combined_cpu_median_percent_one_logical_cpu": statistics.median(combined),
            "processes": summaries,
        },
        "samples": samples,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(record["summary"], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
