#!/usr/bin/env python3
"""Focused Linux regression check for idle daemon blocking behavior."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time


def process_sample(pid: int) -> tuple[int, int, int]:
    stat = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8").split()
    status = Path(f"/proc/{pid}/status").read_text(encoding="utf-8").splitlines()
    voluntary = next(
        int(line.split()[1])
        for line in status
        if line.startswith("voluntary_ctxt_switches:")
    )
    return int(stat[13]) + int(stat[14]), voluntary, time.monotonic_ns()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/debug/yoctui"))
    # At Linux's usual 100 Hz accounting granularity, five seconds quantizes
    # this 0.5% bound in 0.2-point steps and makes one extra tick fail the gate.
    # Ten seconds halves that quantization without weakening the bound.
    parser.add_argument("--sample-seconds", type=float, default=10.0)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    if args.sample_seconds < 2.0:
        raise SystemExit("sample window must be at least two seconds")

    root = Path(tempfile.mkdtemp(prefix="yoctui-idle-event-loop-"))
    runtime = root / "runtime"
    state = root / "state"
    runtime.mkdir(mode=0o700)
    state.mkdir(mode=0o700)
    environment = os.environ.copy()
    environment.pop("BUILDDIR", None)
    environment["XDG_RUNTIME_DIR"] = str(runtime)
    environment["XDG_STATE_HOME"] = str(state)
    process = subprocess.Popen(
        [str(binary), "daemon", "foreground"],
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        socket = runtime / "yoctui" / "daemon.sock"
        deadline = time.monotonic() + 5.0
        while not socket.exists() and process.poll() is None and time.monotonic() < deadline:
            time.sleep(0.01)
        if process.poll() is not None:
            stderr = process.stderr.read() if process.stderr else ""
            raise SystemExit(f"daemon exited during startup: {stderr.strip()}")
        if not socket.exists():
            raise SystemExit("daemon socket was not ready within five seconds")

        before_cpu, before_switches, before_ns = process_sample(process.pid)
        time.sleep(args.sample_seconds)
        after_cpu, after_switches, after_ns = process_sample(process.pid)
        elapsed = (after_ns - before_ns) / 1_000_000_000
        clock_ticks = os.sysconf("SC_CLK_TCK")
        cpu_percent = ((after_cpu - before_cpu) / clock_ticks) / elapsed * 100.0
        voluntary_per_second = (after_switches - before_switches) / elapsed

        shutdown_started = time.monotonic()
        process.send_signal(signal.SIGTERM)
        process.wait(timeout=0.5)
        shutdown_ms = (time.monotonic() - shutdown_started) * 1000.0
        result = {
            "schema": "yoctui.performance.idle-event-loop.v1",
            "sample_seconds": round(elapsed, 3),
            "daemon_cpu_percent_of_one_logical_cpu": round(cpu_percent, 4),
            "leader_voluntary_context_switches_per_second": round(
                voluntary_per_second, 3
            ),
            "shutdown_latency_ms": round(shutdown_ms, 3),
            "bounds": {
                "maximum_cpu_percent": 0.5,
                "maximum_voluntary_context_switches_per_second": 35.0,
                "maximum_shutdown_latency_ms": 500.0,
            },
        }
        print(json.dumps(result, sort_keys=True))
        if cpu_percent > 0.5:
            raise SystemExit(f"idle daemon used {cpu_percent:.3f}% CPU; expected <=0.5%")
        if voluntary_per_second > 35.0:
            raise SystemExit(
                "idle daemon woke too often: "
                f"{voluntary_per_second:.1f} voluntary context switches/s"
            )
        if shutdown_ms > 500.0:
            raise SystemExit(f"daemon shutdown took {shutdown_ms:.1f} ms")
        return 0
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=1.0)
        shutil.rmtree(root)


if __name__ == "__main__":
    raise SystemExit(main())
