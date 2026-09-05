#!/usr/bin/env python3
"""Measure Yoctui's Tokio/runtime footprint without a Yocto workspace."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time


SCHEMA = "yoctui.performance.tokio-runtime.v1"


def task_snapshot(pid: int) -> list[dict[str, object]]:
    tasks = []
    for task in sorted(Path(f"/proc/{pid}/task").iterdir(), key=lambda path: int(path.name)):
        status = task.joinpath("status").read_text(encoding="utf-8").splitlines()
        switches = {
            line.split(":", 1)[0]: int(line.split()[1])
            for line in status
            if line.startswith(("voluntary_ctxt_switches:", "nonvoluntary_ctxt_switches:"))
        }
        stat = task.joinpath("stat").read_text(encoding="utf-8").split()
        tasks.append(
            {
                "tid": int(task.name),
                "name": task.joinpath("comm").read_text(encoding="utf-8").strip(),
                "cpu_ticks": int(stat[13]) + int(stat[14]),
                "voluntary_context_switches": switches["voluntary_ctxt_switches"],
                "involuntary_context_switches": switches[
                    "nonvoluntary_ctxt_switches"
                ],
            }
        )
    return tasks


def source_inventory(root: Path) -> dict[str, int]:
    sources = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted(root.joinpath("crates").glob("**/*.rs"))
    )
    return {
        "tokio_spawn_sites": sources.count("tokio::spawn("),
        "spawn_blocking_sites": sources.count("spawn_blocking("),
        "bounded_mpsc_channel_sites": sources.count("mpsc::channel("),
        "unbounded_mpsc_channel_sites": sources.count("mpsc::unbounded_channel("),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--sample-seconds", type=float, default=2.0)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if len(args.revision) != 40:
        parser.error("revision must be a full 40-character Git commit")
    if args.sample_seconds < 1:
        parser.error("sample window must be at least one second")

    root = Path(__file__).resolve().parents[1]
    binary = args.binary.resolve(strict=True)
    fixture = Path(tempfile.mkdtemp(prefix="yoctui-tokio-audit-"))
    runtime = fixture / "runtime"
    state = fixture / "state"
    runtime.mkdir(mode=0o700)
    state.mkdir(mode=0o700)
    environment = os.environ.copy()
    environment.pop("BUILDDIR", None)
    environment["XDG_RUNTIME_DIR"] = str(runtime)
    environment["XDG_STATE_HOME"] = str(state)
    process = subprocess.Popen(
        [str(binary), "daemon", "foreground"],
        cwd=root,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        socket = runtime / "yoctui" / "daemon.sock"
        deadline = time.monotonic() + 5
        while not socket.exists() and process.poll() is None and time.monotonic() < deadline:
            time.sleep(0.01)
        if process.poll() is not None or not socket.exists():
            stderr = process.stderr.read() if process.stderr else ""
            raise SystemExit(f"daemon failed to become ready: {stderr.strip()}")

        before = task_snapshot(process.pid)
        started = time.monotonic()
        time.sleep(args.sample_seconds)
        elapsed = time.monotonic() - started
        after = task_snapshot(process.pid)
        before_by_tid = {task["tid"]: task for task in before}
        clock_ticks = os.sysconf("SC_CLK_TCK")
        for task in after:
            prior = before_by_tid.get(task["tid"])
            if prior is None:
                task["cpu_percent_of_one_logical_cpu"] = None
                continue
            ticks = int(task["cpu_ticks"]) - int(prior["cpu_ticks"])
            task["cpu_percent_of_one_logical_cpu"] = round(
                ticks / clock_ticks / elapsed * 100, 4
            )
            task["voluntary_context_switches_per_second"] = round(
                (
                    int(task["voluntary_context_switches"])
                    - int(prior["voluntary_context_switches"])
                )
                / elapsed,
                3,
            )
            task["involuntary_context_switches_per_second"] = round(
                (
                    int(task["involuntary_context_switches"])
                    - int(prior["involuntary_context_switches"])
                )
                / elapsed,
                3,
            )

        record = {
            "schema": SCHEMA,
            "captured_at_unix_ms": int(time.time() * 1000),
            "revision": args.revision,
            "binary": {
                "path": str(binary),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "host": {
                "logical_cpus": os.cpu_count(),
                "affinity_cpus": len(os.sched_getaffinity(0)),
            },
            "measurement": {
                "scenario": "idle-daemon-no-workspace",
                "sample_seconds": round(elapsed, 3),
                "thread_count_start": len(before),
                "thread_count_end": len(after),
                "threads": after,
            },
            "source_inventory": source_inventory(root),
        }
        rendered = json.dumps(record, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            temporary = args.output.with_suffix(args.output.suffix + ".tmp")
            temporary.write_text(rendered, encoding="utf-8")
            temporary.replace(args.output)
        print(rendered, end="")
        return 0
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGTERM)
            try:
                process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=1)
        shutil.rmtree(fixture)


if __name__ == "__main__":
    raise SystemExit(main())
