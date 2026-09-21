#!/usr/bin/env python3
"""Capture sustained real-Poky Yoctui performance evidence."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import pty
import select
import signal
import statistics
import struct
import subprocess
import tempfile
import termios
import threading
import time
from datetime import datetime, timezone


ROOT = Path(__file__).resolve().parents[1]
IPC_SCRIPT = ROOT / "scripts/measure-ipc-latency.py"
SPEC = importlib.util.spec_from_file_location("ipc_measure", IPC_SCRIPT)
assert SPEC is not None and SPEC.loader is not None
IPC = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IPC)


def proc(pid: int) -> dict[str, int]:
    stat = Path(f"/proc/{pid}/stat").read_text().split()
    status = Path(f"/proc/{pid}/status").read_text().splitlines()
    fields = {line.split(":", 1)[0]: line.split(":", 1)[1].strip() for line in status if ":" in line}
    return {
        "ticks": int(stat[13]) + int(stat[14]),
        "rss_bytes": int(fields["VmRSS"].split()[0]) * 1024,
        "threads": int(fields["Threads"]),
        "start_ticks": int(stat[21]),
    }


def process_identity(pid: int) -> dict[str, object]:
    state = proc(pid)
    return {
        "pid": pid,
        "start_time_ticks_since_boot": state["start_ticks"],
        "executable": str(Path(f"/proc/{pid}/exe").resolve()),
        "command": Path(f"/proc/{pid}/cmdline")
        .read_bytes()
        .rstrip(b"\0")
        .replace(b"\0", b" ")
        .decode(errors="replace"),
    }


def host_cpu() -> tuple[int, int]:
    fields = [int(value) for value in Path("/proc/stat").read_text().splitlines()[0].split()[1:]]
    idle = fields[3] + fields[4]
    return sum(fields), sum(fields) - idle


def cpu_model() -> str:
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith("model name"):
            return line.split(":", 1)[1].strip()
    return "unknown"


def memory_total_bytes() -> int:
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("MemTotal is absent from /proc/meminfo")


def host_identity(build_dir: Path) -> dict[str, object]:
    filesystem = os.statvfs(build_dir)
    filesystem_type = subprocess.check_output(
        ["findmnt", "-n", "-o", "FSTYPE", "-T", str(build_dir)], text=True
    ).strip()
    return {
        "kernel": platform.release(),
        "machine": platform.machine(),
        "cpu_model": cpu_model(),
        "logical_cpus": os.cpu_count(),
        "online_cpus": Path("/sys/devices/system/cpu/online").read_text().strip(),
        "memory_total_bytes": memory_total_bytes(),
        "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
        "load_average": list(os.getloadavg()),
        "filesystem": {
            "path": str(build_dir),
            "type": filesystem_type,
            "free_bytes": filesystem.f_bavail * filesystem.f_frsize,
            "total_bytes": filesystem.f_blocks * filesystem.f_frsize,
        },
    }


def bitbake_processes(minimum_start_ticks: int) -> dict[int, dict[str, int]]:
    candidates: dict[int, tuple[int, str, dict[str, int]]] = {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            command = (entry / "cmdline").read_bytes().replace(b"\0", b" ").decode(errors="replace")
            state = proc(int(entry.name))
            if state["start_ticks"] >= minimum_start_ticks:
                stat = (entry / "stat").read_text().split()
                candidates[int(entry.name)] = (int(stat[3]), command, state)
        except (FileNotFoundError, KeyError, PermissionError, ProcessLookupError, ValueError):
            continue
    selected = {
        pid
        for pid, (_, command, _) in candidates.items()
        if "bitbake-server" in command or "bitbake-worker" in command
    }
    changed = True
    while changed:
        changed = False
        for pid, (parent, _, _) in candidates.items():
            if pid not in selected and parent in selected:
                selected.add(pid)
                changed = True
    return {pid: candidates[pid][2] for pid in selected}


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, max(0, int(len(ordered) * fraction + 0.999999) - 1))]


def trimmed_mean(values: list[float]) -> float:
    ordered = sorted(values)
    trim = len(ordered) // 10
    retained = ordered[trim:len(ordered) - trim] if trim else ordered
    return statistics.fmean(retained)


def stop(process: subprocess.Popen[object] | None) -> None:
    if process is None or process.poll() is not None:
        return
    process.send_signal(signal.SIGTERM)
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def wait_for_terminal_job(
    observation: object, job_id: int, timeout: float = 600.0
) -> dict[str, object]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = observation.job_states.get(job_id)
        if state is not None and state.get("lifecycle") in {
            "exited",
            "failed",
            "lost",
            "disconnected",
        }:
            return state
        received = observation.client.receive(min(0.5, deadline - time.monotonic()))
        if received is not None:
            observation.process(received[0], received[1])
    raise RuntimeError(f"timed out waiting for preparatory job {job_id}")


def wait_for_task_started(
    observation: object, job_id: int, recipe: str, task: str, timeout: float = 900.0
) -> dict[str, object]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        received = observation.client.receive(min(0.5, deadline - time.monotonic()))
        if received is None:
            continue
        message, size = received
        observation.process(message, size)
        if message.get("type") == "event":
            event = message.get("event", {})
            data = event.get("data", {})
            if (
                event.get("type") == "build"
                and data.get("type") == "task_started"
                and data.get("recipe") == recipe
                and data.get("task") == task
            ):
                return data
        state = observation.job_states.get(job_id)
        if state is not None and state.get("lifecycle") in {
            "exited",
            "failed",
            "lost",
            "disconnected",
        }:
            raise RuntimeError(
                f"real Poky job became terminal before {recipe}:{task} started: {state}"
            )
    raise RuntimeError(f"timed out waiting for real task {recipe}:{task}")


def git_identity(path: Path) -> dict[str, object]:
    status = subprocess.check_output(
        ["git", "status", "--short"], cwd=path, text=True
    )
    diff = subprocess.check_output(["git", "diff", "--binary"], cwd=path)
    return {
        "path": str(path),
        "revision": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=path, text=True
        ).strip(),
        "describe": subprocess.check_output(
            ["git", "describe", "--tags", "--always", "--dirty"], cwd=path, text=True
        ).strip(),
        "dirty": bool(status),
        "status": status.splitlines(),
        "working_diff_sha256": hashlib.sha256(diff).hexdigest(),
    }


def wait_for_initial_workspace(client, snapshot, timeout=300.0):
    """Read-only readiness, outside measurement; never guess missing metadata."""
    instance = snapshot.get("daemon_instance_id")

    def workspace_variables(event):
        if event.get("type") != "workspace":
            return None
        variables = event.get("data", {}).get("variables", {})
        if not isinstance(variables, dict) or any(
            not isinstance(variables.get(key), str) or not variables[key].strip()
            for key in ("BB_NUMBER_THREADS", "PARALLEL_MAKE")
        ):
            raise RuntimeError("daemon workspace omitted BitBake parallelism variables")
        return {key: value for key, value in variables.items() if isinstance(value, str)}

    def check_log(log):
        message = log.get("message", "")
        if isinstance(message, str) and message.startswith("Initial metadata "):
            raise RuntimeError(f"real-Poky startup metadata failed: {message[:1024]}")

    def from_snapshot(current):
        if current.get("daemon_instance_id") != instance:
            raise RuntimeError("real-Poky daemon instance changed during metadata startup")
        for event in current.get("build_events", []):
            variables = workspace_variables(event)
            if variables is not None:
                return variables
        for log in current.get("recent_logs", []):
            check_log(log)
        return None

    variables = from_snapshot(snapshot)
    if variables is not None:
        return variables
    deadline = time.monotonic() + timeout
    remaining_messages = 8192
    while remaining_messages:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError("real-Poky initial workspace readiness timed out")
        received = client.receive(min(0.5, remaining))
        if received is None:
            continue
        remaining_messages -= 1
        message = received[0]
        kind = message.get("type")
        if kind == "ping":
            client.send({"type": "pong", "nonce": message["nonce"]})
        elif kind in {"snapshot", "attached"}:
            variables = from_snapshot(message["snapshot"] if kind == "attached" else message)
        elif kind == "event":
            event = message.get("event", {})
            if event.get("type") == "build":
                variables = workspace_variables(event.get("data", {}))
            elif event.get("type") == "log":
                check_log(event.get("data", {}))
        elif kind in {"error", "resync_required", "detaching", "shutting_down"}:
            raise RuntimeError(f"real-Poky metadata startup interrupted: {str(message)[:1024]}")
        if variables is not None:
            return variables
    raise RuntimeError("real-Poky initial workspace readiness exceeded the message bound")
