#!/usr/bin/env python3
"""Measure the release idle daemon and attached-client CPU contract."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import shutil
import signal
import struct
import subprocess
import tempfile
import termios
import threading
import time


SCHEMA = "yoctui.performance.low-overhead-suite.v1"
ROOT = Path(__file__).resolve().parents[1]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def wait_for_socket(process: subprocess.Popen[bytes], socket_path: Path) -> float:
    started = time.monotonic()
    deadline = started + 5.0
    while time.monotonic() < deadline:
        if socket_path.exists():
            return (time.monotonic() - started) * 1_000
        if process.poll() is not None:
            raise RuntimeError("daemon exited before exposing its socket")
        time.sleep(0.01)
    raise RuntimeError("daemon did not expose its socket within five seconds")


def start_drain(master: int) -> tuple[threading.Event, threading.Thread, list[int]]:
    stopped = threading.Event()
    drained = [0]

    def drain() -> None:
        while not stopped.is_set():
            readable, _, _ = select.select([master], [], [], 0.25)
            if not readable:
                continue
            try:
                data = os.read(master, 65_536)
            except (BlockingIOError, OSError):
                break
            if not data:
                break
            drained[0] += len(data)

    thread = threading.Thread(target=drain, name="yoctui-measurement-pty-drain")
    thread.start()
    return stopped, thread, drained


def stop_process(process: subprocess.Popen[bytes], timeout: float = 2.0) -> None:
    if process.poll() is None:
        process.send_signal(signal.SIGTERM)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=timeout)


def measure(
    binary: Path,
    output: Path,
    scenario: str,
    processes: dict[str, int],
    warmup: int,
    samples: int,
) -> None:
    command = [
        str(ROOT / "scripts/measure-process-overhead.py"),
        "--scenario",
        scenario,
        "--binary",
        str(binary),
        "--warmup-seconds",
        str(warmup),
        "--sample-seconds",
        str(samples),
        "--terminal-columns",
        "160",
        "--terminal-rows",
        "50",
        "--refresh-milliseconds",
        "100",
        "--filesystem-path",
        str(ROOT),
        "--output",
        str(output),
    ]
    for role, pid in processes.items():
        command.extend(["--pid", f"{role}={pid}"])
    subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
    )


def validate_thresholds(
    idle_daemon: dict[str, object], attached: dict[str, object]
) -> dict[str, float]:
    idle_summary = idle_daemon["summary"]
    attached_summary = attached["summary"]
    assert isinstance(idle_summary, dict) and isinstance(attached_summary, dict)
    idle_processes = idle_summary["processes"]
    attached_processes = attached_summary["processes"]
    assert isinstance(idle_processes, dict) and isinstance(attached_processes, dict)
    idle_cpu = float(idle_processes["daemon"]["cpu_trimmed_mean_percent_one_logical_cpu"])
    daemon_attached_cpu = float(
        attached_processes["daemon"]["cpu_trimmed_mean_percent_one_logical_cpu"]
    )
    client_cpu = float(
        attached_processes["client"]["cpu_trimmed_mean_percent_one_logical_cpu"]
    )
    combined_cpu = float(
        attached_summary["combined_cpu_trimmed_mean_percent_one_logical_cpu"]
    )
    values = {
        "idle_daemon_cpu_percent_one_logical_cpu": idle_cpu,
        "attached_daemon_cpu_percent_one_logical_cpu": daemon_attached_cpu,
        "idle_client_cpu_percent_one_logical_cpu": client_cpu,
        "combined_cpu_percent_one_logical_cpu": combined_cpu,
    }
    thresholds = {
        "idle_daemon_cpu_percent_one_logical_cpu": 0.20,
        "idle_client_cpu_percent_one_logical_cpu": 0.50,
        "combined_cpu_percent_one_logical_cpu": 1.00,
    }
    for name, limit in thresholds.items():
        if values[name] > limit:
            raise RuntimeError(f"{name} is {values[name]:.4f}%, above {limit:.2f}%")
    return values


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--warmup-seconds", type=int, default=10)
    parser.add_argument("--sample-seconds", type=int, default=60)
    args = parser.parse_args()
    if len(args.revision) != 40:
        parser.error("revision must be a full 40-character Git commit")
    if args.warmup_seconds < 1 or args.sample_seconds < 10:
        parser.error("warmup must be >=1 second and sample window >=10 seconds")
    binary = args.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error("Yoctui binary is not executable")
    output = args.output_directory.resolve()
    output.mkdir(parents=True, exist_ok=True)

    fixture = Path(tempfile.mkdtemp(prefix="yoctui-low-overhead-"))
    runtime = fixture / "runtime"
    state = fixture / "state"
    config = fixture / "config"
    for directory in (runtime, state, config):
        directory.mkdir(mode=0o700)
    environment = os.environ.copy()
    environment.pop("BUILDDIR", None)
    environment.pop("YOCTUI_BUILD_DIR", None)
    environment.update(
        {
            "XDG_RUNTIME_DIR": str(runtime),
            "XDG_STATE_HOME": str(state),
            "XDG_CONFIG_HOME": str(config),
            "TERM": "xterm-256color",
        }
    )
    daemon = subprocess.Popen(
        [str(binary), "daemon", "foreground"],
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    client: subprocess.Popen[bytes] | None = None
    master: int | None = None
    drain_stop: threading.Event | None = None
    drain_thread: threading.Thread | None = None
    drained = [0]
    try:
        daemon_startup_ms = wait_for_socket(daemon, runtime / "yoctui/daemon.sock")
        idle_path = output / "idle-daemon.json"
        measure(
            binary,
            idle_path,
            "optimized-idle-daemon",
            {"daemon": daemon.pid},
            args.warmup_seconds,
            args.sample_seconds,
        )

        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
        client_started = time.monotonic()
        client = subprocess.Popen(
            [str(binary), "attach"],
            cwd=ROOT,
            env=environment,
            stdin=slave,
            stdout=slave,
            stderr=slave,
            start_new_session=True,
            close_fds=True,
        )
        os.close(slave)
        os.set_blocking(master, False)
        drain_stop, drain_thread, drained = start_drain(master)
        deadline = time.monotonic() + 8.0
        while drained[0] == 0 and client.poll() is None and time.monotonic() < deadline:
            time.sleep(0.01)
        if client.poll() is not None or drained[0] == 0:
            raise RuntimeError("interactive client did not produce its first frame")
        client_startup_ms = (time.monotonic() - client_started) * 1_000

        attached_path = output / "idle-attached-client.json"
        measure(
            binary,
            attached_path,
            "optimized-idle-attached-client",
            {"daemon": daemon.pid, "client": client.pid},
            args.warmup_seconds,
            args.sample_seconds,
        )
        idle_record = json.loads(idle_path.read_text(encoding="utf-8"))
        attached_record = json.loads(attached_path.read_text(encoding="utf-8"))
        values = validate_thresholds(idle_record, attached_record)
        manifest = {
            "schema": SCHEMA,
            "revision": args.revision,
            "captured_at_unix_ms": int(time.time() * 1_000),
            "binary": {
                "path": str(binary),
                "sha256": sha256(binary),
                "version": subprocess.check_output(
                    [str(binary), "--version"], text=True
                ).strip(),
            },
            "sources": {
                str(path.relative_to(ROOT)): sha256(path)
                for path in (
                    ROOT / "crates/yoctui-cli/src/main.rs",
                    ROOT / "crates/yoctui-protocol/src/daemon_ipc.rs",
                    ROOT / "scripts/measure-low-overhead.py",
                    ROOT / "scripts/measure-process-overhead.py",
                )
            },
            "method": {
                "release_profile": True,
                "clock": "CLOCK_MONOTONIC",
                "cpu_source": "/proc/PID/stat fields 14+15",
                "warmup_seconds": args.warmup_seconds,
                "sample_window_seconds": args.sample_seconds,
                "sample_interval_seconds": 1,
                "statistic": "10_percent_trimmed_mean",
                "terminal": {"columns": 160, "rows": 50},
                "refresh_milliseconds": 100,
                "startup_excluded": True,
            },
            "startup": {
                "daemon_socket_ready_ms": daemon_startup_ms,
                "client_first_frame_ms": client_startup_ms,
                "maximum_excluded_startup_ms": 5_000,
            },
            "artifacts": {
                "idle-daemon.json": sha256(idle_path),
                "idle-attached-client.json": sha256(attached_path),
            },
            "thresholds": {
                "idle_daemon_cpu_percent_one_logical_cpu": 0.20,
                "idle_client_cpu_percent_one_logical_cpu": 0.50,
                "combined_cpu_percent_one_logical_cpu": 1.00,
            },
            "observations": values,
            "pty_bytes_drained": drained[0],
        }
        manifest_path = output / "measurement.json"
        manifest_path.write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        print(json.dumps(values, indent=2, sort_keys=True))
        return 0
    finally:
        if client is not None:
            stop_process(client)
        if drain_stop is not None:
            drain_stop.set()
        if drain_thread is not None:
            drain_thread.join(timeout=1.0)
        if master is not None:
            os.close(master)
        stop_process(daemon)
        if daemon.stderr is not None:
            daemon.stderr.close()
        shutil.rmtree(fixture)


if __name__ == "__main__":
    raise SystemExit(main())
