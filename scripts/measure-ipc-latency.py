#!/usr/bin/env python3
"""Measure production daemon IPC latency under deterministic CPU saturation."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import signal
import socket
import statistics
import struct
import subprocess
import tempfile
import time


SCHEMA = "yoctui.performance.ipc-latency.v1"
ROOT = Path(__file__).resolve().parents[1]
BRIDGE = ROOT / "scripts/fixtures/bitbake-ipc-latency-bridge.py"
MARKER = re.compile(r"^PERF_IPC_LATENCY:(\d+):(\d+)$")
EVENT_WARMUP_OBSERVATIONS = 50


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        raise ValueError("percentile requires at least one value")
    ordered = sorted(values)
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)]


def summarize(values: list[float]) -> dict[str, float]:
    return {
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "maximum": max(values),
        "mean": statistics.fmean(values),
    }


def parse_latency_marker(message: str) -> tuple[int, int] | None:
    match = MARKER.fullmatch(message)
    if match is None:
        return None
    return int(match.group(1)), int(match.group(2))


def latency_event(message: dict[str, object]) -> tuple[int, int, int] | None:
    if message.get("type") != "event" or not isinstance(message.get("sequence"), int):
        return None
    event = message.get("event")
    if not isinstance(event, dict) or event.get("type") != "log":
        return None
    data = event.get("data")
    if not isinstance(data, dict) or not isinstance(data.get("message"), str):
        return None
    marker = parse_latency_marker(str(data["message"]))
    if marker is None:
        return None
    fixture_sequence, emitted_ns = marker
    return int(message["sequence"]), fixture_sequence, emitted_ns


class ProtocolClient:
    def __init__(self, socket_path: Path, client_byte: int) -> None:
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(str(socket_path))
        self.client_id = [client_byte] * 16
        self.pending = bytearray()

    def send(self, message: dict[str, object]) -> None:
        payload = json.dumps(message, separators=(",", ":")).encode()
        self.socket.sendall(struct.pack(">I", len(payload)) + payload)

    def receive(self, timeout: float) -> tuple[dict[str, object], int] | None:
        deadline = time.monotonic() + timeout
        while True:
            if len(self.pending) >= 4:
                length = struct.unpack(">I", self.pending[:4])[0]
                if length > 4 * 1024 * 1024:
                    raise RuntimeError("daemon IPC frame exceeded 4 MiB")
                frame_length = length + 4
                if len(self.pending) >= frame_length:
                    payload = bytes(self.pending[4:frame_length])
                    del self.pending[:frame_length]
                    message = json.loads(payload)
                    return message, time.clock_gettime_ns(time.CLOCK_MONOTONIC)
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
                "client_name": "ipc-latency-harness",
                "capabilities": [
                    "state_snapshots",
                    "incremental_events",
                    "event_replay",
                    "background_jobs",
                    "environment_compatibility",
                ],
            }
        )
        hello = self.receive(5.0)
        if hello is None or hello[0].get("type") != "hello":
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
        attached = self.receive(10.0)
        if attached is None or attached[0].get("type") != "attached":
            raise RuntimeError(f"unexpected daemon attach response: {attached}")
        snapshot = attached[0].get("snapshot")
        if not isinstance(snapshot, dict):
            raise RuntimeError("daemon attach omitted its snapshot")
        return snapshot

    def close(self) -> None:
        try:
            self.send({"type": "detach"})
            self.receive(1.0)
        except (BrokenPipeError, EOFError, OSError):
            pass
        self.socket.close()


class Observation:
    def __init__(
        self, client: ProtocolClient, observations: int, event_warmup: int
    ) -> None:
        self.client = client
        self.required = observations
        self.event_warmup = event_warmup
        self.protocol_sequences: list[int] = []
        self.event_samples: list[dict[str, int | float]] = []
        self.backend_disconnects = 0

    def process(self, message: dict[str, object], received_ns: int) -> None:
        if message.get("type") != "event":
            return
        sequence = message.get("sequence")
        if isinstance(sequence, int):
            self.protocol_sequences.append(sequence)
        event = message.get("event")
        if isinstance(event, dict) and event.get("type") == "build":
            data = event.get("data")
            if isinstance(data, dict) and data.get("type") == "disconnected":
                self.backend_disconnects += 1
        marker = latency_event(message)
        if marker is None or len(self.event_samples) >= self.required:
            return
        daemon_sequence, fixture_sequence, emitted_ns = marker
        if fixture_sequence <= self.event_warmup:
            return
        sequence = len(self.event_samples) + 1
        expected_fixture = self.event_warmup + sequence
        if fixture_sequence != expected_fixture:
            raise RuntimeError(
                "timestamped event order changed: "
                f"expected {expected_fixture}, got {fixture_sequence}"
            )
        if emitted_ns > received_ns:
            raise RuntimeError("event timestamp is later than client receipt")
        self.event_samples.append(
            {
                "sequence": sequence,
                "daemon_sequence": daemon_sequence,
                "fixture_sequence": fixture_sequence,
                "emitted_ns": emitted_ns,
                "received_ns": received_ns,
                "latency_ms": (received_ns - emitted_ns) / 1_000_000,
            }
        )

    def receive(self, timeout: float = 2.0) -> dict[str, object]:
        received = self.client.receive(timeout)
        if received is None:
            raise RuntimeError("timed out waiting for daemon IPC")
        message, received_ns = received
        self.process(message, received_ns)
        return message

    def command(
        self, request_id: int, generation: int | None, command: dict[str, object]
    ) -> tuple[dict[str, object], int, int]:
        sent_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
        self.client.send(
            {
                "type": "command",
                "request_id": request_id,
                "expected_generation": generation,
                "command": command,
            }
        )
        deadline = time.monotonic() + 2.0
        while time.monotonic() < deadline:
            received = self.client.receive(max(0.001, deadline - time.monotonic()))
            if received is None:
                break
            message, received_ns = received
            self.process(message, received_ns)
            if message.get("type") == "command_result" and message.get(
                "request_id"
            ) == request_id:
                return message, sent_ns, received_ns
        raise RuntimeError(f"timed out waiting for command result {request_id}")


def accepted(result: dict[str, object]) -> bool:
    outcome = result.get("outcome")
    return isinstance(outcome, dict) and outcome.get("type") == "accepted"


def outcome_identity(result: dict[str, object]) -> tuple[str, str | None]:
    outcome = result.get("outcome")
    if not isinstance(outcome, dict) or not isinstance(outcome.get("type"), str):
        raise RuntimeError(f"command result omitted a typed outcome: {result}")
    code = outcome.get("code")
    return str(outcome["type"]), str(code) if isinstance(code, str) else None


def job_event(message: dict[str, object]) -> dict[str, object] | None:
    if message.get("type") != "event":
        return None
    event = message.get("event")
    if not isinstance(event, dict) or event.get("type") != "job_changed":
        return None
    data = event.get("data")
    return data if isinstance(data, dict) else None


def wait_for_job(
    observation: Observation, job_id: int | None, lifecycles: set[str]
) -> dict[str, object]:
    deadline = time.monotonic() + 3.0
    while time.monotonic() < deadline:
        message = observation.receive(max(0.001, deadline - time.monotonic()))
        job = job_event(message)
        if job is not None and (job_id is None or job.get("id") == job_id):
            if job.get("lifecycle") in lifecycles:
                return job
    raise RuntimeError(f"timed out waiting for job {job_id} in {sorted(lifecycles)}")


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


def start_daemon(binary: Path, root: Path) -> tuple[subprocess.Popen[str], Path]:
    runtime = root / "runtime"
    state = root / "state"
    config = root / "config"
    for directory in (runtime, state, config):
        directory.mkdir(mode=0o700)
    build, binary_dir = write_fake_environment(root)
    environment = os.environ.copy()
    environment.update(
        {
            "BUILDDIR": str(build),
            "YOCTUI_BUILD_DIR": str(build),
            "XDG_RUNTIME_DIR": str(runtime),
            "XDG_STATE_HOME": str(state),
            "XDG_CONFIG_HOME": str(config),
            "YOCTUI_BRIDGE_PATH": str(BRIDGE),
            "YOCTUI_PERF_IPC_EVENT_RATE": "500",
            "PATH": f"{binary_dir}:{environment['PATH']}",
        }
    )
    daemon = subprocess.Popen(
        [str(binary), "daemon", "foreground"],
        cwd=ROOT,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    socket_path = runtime / "yoctui/daemon.sock"
    deadline = time.monotonic() + 20.0
    while time.monotonic() < deadline:
        if socket_path.exists():
            return daemon, socket_path
        if daemon.poll() is not None:
            stdout, stderr = daemon.communicate()
            raise RuntimeError(f"daemon startup failed: {stdout}\n{stderr}")
        time.sleep(0.02)
    raise RuntimeError("daemon did not expose its socket within 20 seconds")


def stop_process(process: subprocess.Popen[str], timeout: float = 5.0) -> None:
    if process.poll() is None:
        process.send_signal(signal.SIGTERM)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=timeout)
    for stream in (process.stdout, process.stderr):
        if stream is not None:
            stream.close()


def wait_saturation_ready(process: subprocess.Popen[str], event_log: Path) -> None:
    deadline = time.monotonic() + 10.0
    while process.poll() is None and time.monotonic() < deadline:
        if event_log.exists() and '"event":"ready"' in event_log.read_text(
            encoding="utf-8"
        ):
            return
        time.sleep(0.02)
    raise RuntimeError("CPU saturation fixture did not become ready")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--warmup-seconds", type=float, default=1.0)
    parser.add_argument("--observations", type=int, default=100)
    args = parser.parse_args()
    if len(args.revision) != 40:
        parser.error("revision must be a full 40-character Git commit")
    if args.warmup_seconds < 0.25:
        parser.error("warmup must be at least 0.25 seconds")
    if args.observations < 100 or args.observations > 1_000:
        parser.error("observations must be within 100..1000")
    binary = args.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error("Yoctui binary is not executable")

    fixture = Path(tempfile.mkdtemp(prefix="yoctui-ipc-latency-"))
    saturation_path = fixture / "saturation.json"
    saturation_events = fixture / "saturation.jsonl"
    daemon: subprocess.Popen[str] | None = None
    saturation: subprocess.Popen[str] | None = None
    client: ProtocolClient | None = None
    observation = None
    command_samples: list[dict[str, int | float]] = []
    cancellation_samples: list[dict[str, int | float]] = []
    reconnect_succeeded = False
    saturation_alive_for_all_samples = True
    try:
        daemon, socket_path = start_daemon(binary, fixture)
        client = ProtocolClient(socket_path, 31)
        snapshot = client.attach()
        generation = snapshot.get("generation")
        if not isinstance(generation, int):
            raise RuntimeError("daemon snapshot omitted generation")
        observation = Observation(
            client, args.observations, EVENT_WARMUP_OBSERVATIONS
        )
        saturation = subprocess.Popen(
            [
                str(ROOT / "scripts/cpu-saturation-harness.py"),
                "--warmup-seconds",
                str(args.warmup_seconds),
                "--duration-seconds",
                "3",
                "--minimum-worker-cpu-percent",
                "25",
                "--event-log",
                str(saturation_events),
                "--output",
                str(saturation_path),
            ],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
        )
        wait_saturation_ready(saturation, saturation_events)
        time.sleep(args.warmup_seconds + 0.1)

        start_result, _, _ = observation.command(
            1,
            None,
            {
                "type": "start_build",
                "targets": ["ipc-latency-fixture"],
                "task": None,
                "force": False,
            },
        )
        if not accepted(start_result):
            raise RuntimeError(f"fixture build was not accepted: {start_result}")
        job = wait_for_job(observation, None, {"connecting", "running"})
        job_id = job.get("id")
        if not isinstance(job_id, int):
            raise RuntimeError("build job event omitted its numeric identity")

        while len(observation.event_samples) < args.observations:
            observation.receive()
            saturation_alive_for_all_samples &= saturation.poll() is None

        for index in range(1, args.observations + 1):
            request_id = 1_000 + index
            result, sent_ns, received_ns = observation.command(
                request_id,
                None,
                {"type": "cancel_job", "job_id": 18_446_744_073_709_551_615},
            )
            outcome, rejection_code = outcome_identity(result)
            if outcome != "rejected" or rejection_code != "not_found":
                raise RuntimeError(f"command receipt probe returned an unexpected result: {result}")
            command_samples.append(
                {
                    "sequence": index,
                    "request_id": request_id,
                    "sent_ns": sent_ns,
                    "acknowledged_ns": received_ns,
                    "latency_ms": (received_ns - sent_ns) / 1_000_000,
                }
            )
            saturation_alive_for_all_samples &= saturation.poll() is None

        accepted_cancellations = 0
        for batch_start in range(1, args.observations + 1, 50):
            batch_end = min(batch_start + 49, args.observations)
            cancellation_sent: dict[int, tuple[int, int]] = {}
            batch_accepted = 0
            for index in range(batch_start, batch_end + 1):
                request_id = 2_000 + index
                sent_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
                client.send(
                    {
                        "type": "command",
                        "request_id": request_id,
                        "expected_generation": None,
                        "command": {"type": "cancel_job", "job_id": job_id},
                    }
                )
                cancellation_sent[request_id] = (index, sent_ns)
            cancellation_deadline = time.monotonic() + 5.0
            while cancellation_sent and time.monotonic() < cancellation_deadline:
                received = client.receive(
                    max(0.001, cancellation_deadline - time.monotonic())
                )
                if received is None:
                    break
                message, received_ns = received
                observation.process(message, received_ns)
                request_id = message.get("request_id")
                if (
                    message.get("type") != "command_result"
                    or request_id not in cancellation_sent
                ):
                    continue
                outcome, rejection_code = outcome_identity(message)
                if outcome == "accepted":
                    batch_accepted += 1
                    accepted_cancellations += 1
                elif outcome != "rejected" or rejection_code != "not_found":
                    raise RuntimeError(f"unexpected cancellation outcome: {message}")
                index, sent_ns = cancellation_sent.pop(int(request_id))
                cancellation_samples.append(
                    {
                        "sequence": index,
                        "request_id": request_id,
                        "job_id": job_id,
                        "sent_ns": sent_ns,
                        "acknowledged_ns": received_ns,
                        "latency_ms": (received_ns - sent_ns) / 1_000_000,
                        "outcome": outcome,
                        "rejection_code": rejection_code,
                    }
                )
                saturation_alive_for_all_samples &= saturation.poll() is None
            if cancellation_sent:
                raise RuntimeError(
                    f"missing {len(cancellation_sent)} cancellation acknowledgements"
                )
            if batch_accepted == 0:
                raise RuntimeError("cancellation batch did not cancel its active build")
            wait_for_job(observation, job_id, {"failed", "exited", "lost"})
            if batch_end == args.observations:
                continue
            next_result, _, _ = observation.command(
                3_000 + batch_end,
                None,
                {
                    "type": "start_build",
                    "targets": ["ipc-latency-fixture"],
                    "task": None,
                    "force": False,
                },
            )
            if not accepted(next_result):
                raise RuntimeError(f"replacement fixture build was not accepted: {next_result}")
            job = wait_for_job(observation, None, {"connecting", "running"})
            job_id = job.get("id")
            if not isinstance(job_id, int):
                raise RuntimeError("replacement build omitted numeric job identity")
        cancellation_samples.sort(key=lambda sample: int(sample["sequence"]))

        probe = ProtocolClient(socket_path, 32)
        probe.attach()
        probe.close()
        reconnect_succeeded = True
        saturation_alive_for_all_samples &= saturation.poll() is None

        saturation_stderr = saturation.communicate(timeout=10.0)[1]
        if saturation.returncode != 0:
            raise RuntimeError(
                f"saturation fixture failed after measurement: {saturation_stderr.strip()}"
            )
        load = json.loads(saturation_path.read_text(encoding="utf-8"))
        event_latencies = [float(sample["latency_ms"]) for sample in observation.event_samples]
        command_latencies = [float(sample["latency_ms"]) for sample in command_samples]
        cancellation_latencies = [
            float(sample["latency_ms"]) for sample in cancellation_samples
        ]
        ordered = all(
            current > previous
            for previous, current in zip(
                observation.protocol_sequences, observation.protocol_sequences[1:]
            )
        )
        record = {
            "schema": SCHEMA,
            "revision": args.revision,
            "captured_at_unix_ms": int(time.time() * 1_000),
            "binary": {
                "path": str(binary),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "host": {
                "logical_cpus": os.cpu_count(),
                "affinity_cpus": sorted(os.sched_getaffinity(0)),
                "kernel": os.uname().release,
            },
            "configuration": {
                "clock": "CLOCK_MONOTONIC",
                "transport": "AF_UNIX SOCK_STREAM length-prefixed JSON",
                "warmup_seconds": args.warmup_seconds,
                "observations_per_path": args.observations,
                "event_warmup_observations": EVENT_WARMUP_OBSERVATIONS,
                "event_rate_per_second": 500,
                "load": "one pinned worker per affinity CPU; no deliberately free CPU",
                "event_path": [
                    "fixture_bridge",
                    "bridge_backend",
                    "daemon_bitbake_supervisor",
                    "daemon_snapshot_journal",
                    "unix_ipc",
                    "attached_protocol_client",
                ],
                "command_method": "client cancel_job for a deliberately absent u64 job identity through daemon dispatch to correlated not_found command_result receipt; measured round trip is a conservative upper bound on daemon receipt without command workload",
                "cancellation_method": "two protocol-compliant batches of 50 pipelined cancel_job requests against active BitBake jobs; every request receives an ordered correlated command_result and each batch proves at least one accepted live-supervisor cancellation",
            },
            "summary": {
                "daemon_event_to_client_ms": summarize(event_latencies),
                "client_command_to_daemon_ms": summarize(command_latencies),
                "cancellation_request_to_ack_ms": summarize(cancellation_latencies),
                "accepted_cancellation_requests": accepted_cancellations,
            },
            "samples": {
                "daemon_event_to_client": observation.event_samples,
                "client_command_to_daemon": command_samples,
                "cancellation_request_to_ack": cancellation_samples,
            },
            "continuity": {
                "initial_generation": generation,
                "primary_client_connected": True,
                "reconnect_succeeded": reconnect_succeeded,
                "backend_disconnect_events": observation.backend_disconnects,
                "protocol_sequences_strictly_increasing": ordered,
                "protocol_event_count": len(observation.protocol_sequences),
            },
            "saturation": {
                "alive_for_every_observation": saturation_alive_for_all_samples,
                "completed_after_measurement": load["status"] == "completed",
                "configuration": load["configuration"],
                "achieved": load["achieved"],
                "cleanup": load["cleanup"],
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
        if client is not None:
            client.close()
        if saturation is not None and saturation.poll() is None:
            stop_process(saturation)
        if daemon is not None:
            stop_process(daemon)
        shutil.rmtree(fixture)


if __name__ == "__main__":
    raise SystemExit(main())
