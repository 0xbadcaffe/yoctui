#!/usr/bin/env python3
"""Exercise the production daemon/bridge/IPC path with a deterministic event flood."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import tempfile
import time


SCHEMA = "yoctui.performance.event-flood-observation.v1"
ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "scripts/fixtures/bitbake-event-flood-bridge.py"
CRITICAL_NAMES = {
    "warning_sentinel",
    "error_sentinel",
    "critical_task_queued",
    "critical_task_started",
    "critical_task_progress",
    "critical_task_failed",
    "build_terminal",
}


class ProtocolClient:
    def __init__(
        self, socket_path: Path, client_byte: int = 9, receive_buffer_bytes: int | None = None
    ) -> None:
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        if receive_buffer_bytes is not None:
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, receive_buffer_bytes)
        self.socket.settimeout(0.25)
        self.socket.connect(str(socket_path))
        self.client_id = [client_byte] * 16
        self.pending = bytearray()
        self.frames_sent = 0
        self.frame_bytes_sent = 0
        self.frames_received = 0
        self.frame_bytes_received = 0
        self.received_by_type: dict[str, dict[str, int]] = {}

    def send(self, message: dict[str, object]) -> None:
        payload = json.dumps(message, separators=(",", ":")).encode()
        self.socket.sendall(struct.pack(">I", len(payload)) + payload)
        self.frames_sent += 1
        self.frame_bytes_sent += len(payload) + 4

    def receive(self, timeout: float = 0.25) -> dict[str, object] | None:
        deadline = time.monotonic() + timeout
        while True:
            if len(self.pending) >= 4:
                length = struct.unpack(">I", self.pending[:4])[0]
                if length > 4 * 1024 * 1024:
                    raise RuntimeError(f"daemon frame exceeds protocol bound: {length}")
                frame_length = length + 4
                if len(self.pending) >= frame_length:
                    payload = bytes(self.pending[4:frame_length])
                    del self.pending[:frame_length]
                    message = json.loads(payload)
                    self.frames_received += 1
                    self.frame_bytes_received += frame_length
                    kind = str(message.get("type", "unknown"))
                    metrics = self.received_by_type.setdefault(
                        kind,
                        {"frames": 0, "frame_bytes": 0, "minimum_frame_bytes": frame_length,
                         "maximum_frame_bytes": frame_length},
                    )
                    metrics["frames"] += 1
                    metrics["frame_bytes"] += frame_length
                    metrics["minimum_frame_bytes"] = min(
                        metrics["minimum_frame_bytes"], frame_length
                    )
                    metrics["maximum_frame_bytes"] = max(
                        metrics["maximum_frame_bytes"], frame_length
                    )
                    return message
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return None
            self.socket.settimeout(remaining)
            try:
                chunk = self.socket.recv(65_536)
            except socket.timeout:
                return None
            if not chunk:
                raise EOFError("daemon IPC disconnected")
            self.pending.extend(chunk)

    def attach(self) -> dict[str, object]:
        self.send(
            {
                "type": "hello",
                "minimum_version": {"major": 1, "minor": 2},
                "maximum_version": {"major": 1, "minor": 2},
                "client_id": self.client_id,
                "client_name": "event-flood-harness",
                "capabilities": [
                    "state_snapshots",
                    "incremental_events",
                    "event_replay",
                    "background_jobs",
                    "environment_compatibility",
                ],
            }
        )
        hello = self.receive(5)
        if hello is None or hello.get("type") != "hello":
            raise RuntimeError(f"unexpected daemon hello: {hello}")
        self.send(
            {
                "type": "attach",
                "workspace": None,
                "subscription": {
                    "state": True,
                    "jobs": True,
                    "logs": True,
                    "pty_sessions": [],
                },
                "resume": None,
            }
        )
        attached = self.receive(10)
        if attached is None or attached.get("type") != "attached":
            raise RuntimeError(f"unexpected daemon attach response: {attached}")
        snapshot = attached.get("snapshot")
        if not isinstance(snapshot, dict):
            raise RuntimeError("daemon attach omitted snapshot")
        return snapshot

    def close(self) -> None:
        try:
            self.send({"type": "detach"})
            self.receive(1)
        except (BrokenPipeError, EOFError, OSError, TimeoutError):
            pass
        self.socket.close()


def process_rss(pid: int) -> int | None:
    try:
        fields = Path(f"/proc/{pid}/statm").read_text(encoding="utf-8").split()
        return int(fields[1]) * os.sysconf("SC_PAGE_SIZE")
    except (FileNotFoundError, IndexError, ValueError):
        return None


def process_cpu_seconds(pid: int) -> float | None:
    try:
        fields = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8").split()
        ticks = int(fields[13]) + int(fields[14])
        return ticks / os.sysconf("SC_CLK_TCK")
    except (FileNotFoundError, IndexError, ValueError):
        return None


def classify_message(message: dict[str, object], observed: set[str]) -> None:
    if message.get("type") in {"snapshot", "attached"}:
        snapshot = message.get("snapshot")
        if isinstance(snapshot, dict):
            classify_snapshot(snapshot, observed)
        return
    if message.get("type") != "event":
        return
    event = message.get("event")
    if not isinstance(event, dict):
        return
    kind = event.get("type")
    data = event.get("data")
    if kind == "log" and isinstance(data, dict):
        classify_log(data, observed)
    elif kind == "build" and isinstance(data, dict):
        classify_build(data, observed)


def observe_pressure(
    message: dict[str, object], observed: dict[str, int]
) -> None:
    if message.get("type") != "event":
        return
    event = message.get("event")
    if not isinstance(event, dict) or event.get("type") != "telemetry":
        return
    data = event.get("data")
    if not isinstance(data, dict):
        return
    pressure = data.get("pressure")
    if not isinstance(pressure, dict):
        return
    for key, value in pressure.items():
        if isinstance(value, int):
            observed[key] = max(observed.get(key, 0), value)


def classify_log(record: dict[str, object], observed: set[str]) -> None:
    message = record.get("message")
    if message == "PERF_CRITICAL_WARNING":
        observed.add("warning_sentinel")
    elif message == "PERF_CRITICAL_ERROR":
        observed.add("error_sentinel")
    elif message == "PERF_CRITICAL_CANCELLATION":
        observed.add("cancellation")


def classify_build(event: dict[str, object], observed: set[str]) -> None:
    kind = event.get("type")
    if event.get("recipe") == "perf-critical" and event.get("task") == "do_failure":
        mapping = {
            "task_queued": "critical_task_queued",
            "task_started": "critical_task_started",
            "task_progress": "critical_task_progress",
            "task_completed": "critical_task_failed",
        }
        if kind in mapping:
            observed.add(mapping[str(kind)])
    if kind == "completed":
        observed.add("build_terminal")
    elif kind == "disconnected":
        observed.add("backend_disconnect")


def classify_snapshot(snapshot: dict[str, object], observed: set[str]) -> None:
    for record in snapshot.get("recent_logs", []):
        if isinstance(record, dict):
            classify_log(record, observed)
    for event in snapshot.get("build_events", []):
        if isinstance(event, dict):
            classify_build(event, observed)


def write_fake_environment(root: Path) -> tuple[Path, Path]:
    build = root / "build"
    binary_dir = root / "bin"
    (build / "conf").mkdir(parents=True)
    binary_dir.mkdir()
    (build / "conf/local.conf").write_text(
        'MACHINE = "qemux86-64"\nDISTRO = "poky"\n', encoding="utf-8"
    )
    (build / "conf/bblayers.conf").write_text('BBLAYERS = ""\n', encoding="utf-8")
    bitbake = binary_dir / "bitbake"
    bitbake.write_text(
        "#!/bin/sh\n"
        'if [ "${1:-}" = --version ]; then\n'
        "  echo 'BitBake Build Tool Core version 2.18.0'\n"
        "else\n"
        "  echo 'usage: bitbake [options] target'\n"
        "fi\n",
        encoding="utf-8",
    )
    bitbake.chmod(0o755)
    return build, binary_dir


def start_daemon(
    binary: Path, root: Path, rate: int, duration: float
) -> tuple[subprocess.Popen[str], Path, Path]:
    runtime = root / "runtime"
    state = root / "state"
    config = root / "config"
    for directory in (runtime, state, config):
        directory.mkdir(mode=0o700)
    build, binary_dir = write_fake_environment(root)
    report = root / "generator.json"
    environment = os.environ.copy()
    environment.update(
        {
            "BUILDDIR": str(build),
            "YOCTUI_BUILD_DIR": str(build),
            "XDG_RUNTIME_DIR": str(runtime),
            "XDG_STATE_HOME": str(state),
            "XDG_CONFIG_HOME": str(config),
            "YOCTUI_BRIDGE_PATH": str(FIXTURE),
            "YOCTUI_PERF_EVENT_RATE": str(rate),
            "YOCTUI_PERF_EVENT_DURATION": str(duration),
            "YOCTUI_PERF_EVENT_PROFILE": "balanced",
            "YOCTUI_PERF_EVENT_TERMINAL": "success",
            "YOCTUI_PERF_EVENT_REPORT": str(report),
            "PATH": f"{binary_dir}:{environment['PATH']}",
        }
    )
    daemon = subprocess.Popen(
        [str(binary), "daemon", "foreground"],
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    socket_path = runtime / "yoctui/daemon.sock"
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if socket_path.exists():
            return daemon, socket_path, report
        if daemon.poll() is not None:
            stdout, stderr = daemon.communicate()
            raise RuntimeError(f"daemon startup failed: {stdout}\n{stderr}")
        time.sleep(0.02)
    raise RuntimeError("daemon did not expose its socket within 20 seconds")


def stop_daemon(daemon: subprocess.Popen[str]) -> None:
    if daemon.poll() is None:
        daemon.send_signal(signal.SIGTERM)
        try:
            daemon.wait(timeout=5)
        except subprocess.TimeoutExpired:
            daemon.kill()
            daemon.wait(timeout=5)
    for stream in (daemon.stdout, daemon.stderr):
        if stream is not None:
            stream.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/yoctui")
    parser.add_argument("--rate", type=int, default=4_000)
    parser.add_argument("--duration-seconds", type=float, default=1.0)
    parser.add_argument("--observation-seconds", type=float, default=1.5)
    parser.add_argument("--expect-pre-backpressure-failure", action="store_true")
    parser.add_argument("--include-slow-client", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if (
        args.rate < 2_000
        or args.duration_seconds < 0.1
        or args.observation_seconds <= 0
    ):
        parser.error("rate must be >=2000 and durations must be positive")
    binary = args.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error(f"Yoctui binary is not executable: {binary}")

    begun = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="yoctui-event-flood-") as directory:
        root = Path(directory)
        daemon, socket_path, generator_report = start_daemon(
            binary, root, args.rate, args.duration_seconds
        )
        observed: set[str] = set()
        observed_pressure: dict[str, int] = {}
        frame_count = 0
        snapshots = 0
        resyncs = 0
        sequences: list[int] = []
        rss_samples: list[int] = []
        client_continuity = False
        generated: dict[str, object] | None = None
        slow_client: ProtocolClient | None = None
        try:
            client = ProtocolClient(socket_path)
            initial = client.attach()
            if args.include_slow_client:
                slow_client = ProtocolClient(socket_path, 11, receive_buffer_bytes=4_096)
                slow_client.attach()
            initial_snapshot_bytes = len(
                json.dumps(initial, separators=(",", ":")).encode()
            )
            classify_snapshot(initial, observed)
            generation = initial.get("generation")
            measurement_started = time.monotonic()
            daemon_cpu_started = process_cpu_seconds(daemon.pid)
            client.send(
                {
                    "type": "command",
                    "request_id": 1,
                    "expected_generation": generation,
                    "command": {
                        "type": "start_build",
                        "targets": ["event-flood-fixture"],
                        "task": None,
                        "force": False,
                    },
                }
            )
            report_seen_at: float | None = None
            absolute_deadline = (
                time.monotonic() + args.duration_seconds + args.observation_seconds + 10
            )
            while time.monotonic() < absolute_deadline:
                rss = process_rss(daemon.pid)
                if rss is not None:
                    rss_samples.append(rss)
                message = client.receive(0.05)
                if message is not None:
                    frame_count += 1
                    if message.get("type") == "snapshot":
                        snapshots += 1
                    elif message.get("type") == "resync_required":
                        resyncs += 1
                    sequence = message.get("sequence")
                    if isinstance(sequence, int):
                        sequences.append(sequence)
                    classify_message(message, observed)
                    observe_pressure(message, observed_pressure)
                if generator_report.exists() and report_seen_at is None:
                    report_seen_at = time.monotonic()
                if (
                    report_seen_at is not None
                    and time.monotonic() - report_seen_at >= args.observation_seconds
                ):
                    break
            measurement_elapsed = max(time.monotonic() - measurement_started, 0.000_001)
            daemon_cpu_finished = process_cpu_seconds(daemon.pid)
            wire_metrics = {
                "measurement_seconds": measurement_elapsed,
                "frames_received": client.frames_received,
                "frame_bytes_received": client.frame_bytes_received,
                "frames_per_second": client.frames_received / measurement_elapsed,
                "bytes_per_second": client.frame_bytes_received / measurement_elapsed,
                "frames_sent": client.frames_sent,
                "frame_bytes_sent": client.frame_bytes_sent,
                "initial_snapshot_json_bytes": initial_snapshot_bytes,
                "received_by_type": client.received_by_type,
                "daemon_cpu_seconds": (
                    daemon_cpu_finished - daemon_cpu_started
                    if daemon_cpu_started is not None and daemon_cpu_finished is not None
                    else None
                ),
            }
            probe = ProtocolClient(socket_path, 10)
            probe_snapshot = probe.attach()
            classify_snapshot(probe_snapshot, observed)
            client_continuity = True
            probe.close()
            client.close()
            if not generator_report.exists():
                raise RuntimeError("event generator did not publish its bounded report")
            generated = json.loads(generator_report.read_text(encoding="utf-8"))
        finally:
            if slow_client is not None:
                slow_client.socket.close()
            stop_daemon(daemon)
        if generated is None:
            raise RuntimeError("event generator report was not loaded")

        sent_names = {entry["name"] for entry in generated["critical_sent"]}
        critical_received = sorted(sent_names & observed)
        missing = sorted(sent_names - observed)
        ordered_sequences = all(
            current > previous for previous, current in zip(sequences, sequences[1:])
        )
        retention_passed = CRITICAL_NAMES.issubset(observed)
        known_failure = (
            "build_terminal" in sent_names
            and "build_terminal" not in observed
            and client_continuity
        )
        record = {
            "schema": SCHEMA,
            "status": "observed",
            "identity": {
                "source_base_revision": subprocess.check_output(
                    ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
                ).strip(),
                "binary_path": str(binary),
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "configuration": {
                "rate_events_per_second": args.rate,
                "duration_seconds": args.duration_seconds,
                "observation_seconds_after_generator_terminal": args.observation_seconds,
                "production_path": [
                    "fixture_bridge",
                    "bridge_backend",
                    "daemon_bitbake_supervisor",
                    "daemon_reducer_journal",
                    "unix_ipc",
                    "attached_protocol_client",
                ],
                "slow_client_enabled": args.include_slow_client,
            },
            "generator": generated,
            "client": {
                "frames_received": frame_count,
                "snapshot_replacements": snapshots,
                "resync_requests": resyncs,
                "event_sequences_strictly_increasing": ordered_sequences,
                "connection_continuity": client_continuity,
                "reconnect_probe_succeeded": client_continuity,
                "critical_received": critical_received,
                "critical_missing": missing,
                "wire_metrics": wire_metrics,
                "pressure": observed_pressure,
            },
            "bounds": {
                "daemon_rss_initial_bytes": rss_samples[0] if rss_samples else None,
                "daemon_rss_max_bytes": max(rss_samples, default=None),
                "daemon_rss_final_bytes": rss_samples[-1] if rss_samples else None,
                "journal_retained_events": 4096,
                "snapshot_build_events": 2048,
                "snapshot_recent_logs": 512,
                "supervisor_ingress": "bounded_priority_lanes",
                "supervisor_reliable_events": 512,
                "supervisor_cosmetic_events": 512,
                "per_client_backlog_events": 4_096,
                "slow_client_write_deadline_milliseconds": 2,
            },
            "result": {
                "critical_retention_passed": retention_passed,
                "expected_pre_backpressure_terminal_starvation_observed": known_failure,
                "runtime_seconds": time.monotonic() - begun,
            },
        }
        rendered = json.dumps(record, indent=2) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        if not ordered_sequences or not client_continuity:
            return 1
        if args.expect_pre_backpressure_failure:
            return 0 if known_failure and not retention_passed else 1
        return 0 if retention_passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
