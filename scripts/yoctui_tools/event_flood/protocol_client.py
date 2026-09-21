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
import sys
import tempfile
import time


SCHEMA = "yoctui.performance.event-flood-observation.v1"
ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "scripts/fixtures/bitbake-event-flood-bridge.py"
CRITICAL_NAMES = {
    "warning_sentinel",
    "error_sentinel",
    "critical_task_failed",
    "build_terminal",
}
IMPORTANT_TRANSITION_NAMES = {
    "critical_task_queued",
    "critical_task_started",
}
COALESCIBLE_NAMES = {"critical_task_progress"}


class ProtocolClient:
    def __init__(
        self,
        socket_path: Path,
        client_byte: int = 9,
        receive_buffer_bytes: int | None = None,
    ) -> None:
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        if receive_buffer_bytes is not None:
            self.socket.setsockopt(
                socket.SOL_SOCKET, socket.SO_RCVBUF, receive_buffer_bytes
            )
        self.socket.settimeout(0.25)
        self.socket.connect(str(socket_path))
        self.client_id = [client_byte] * 16
        self.pending = bytearray()
        self.frames_sent = 0
        self.frame_bytes_sent = 0
        self.frames_received = 0
        self.frame_bytes_received = 0
        self.received_by_type: dict[str, dict[str, int]] = {}
        self.daemon_instance_id: list[int] | None = None

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
                        {
                            "frames": 0,
                            "frame_bytes": 0,
                            "minimum_frame_bytes": frame_length,
                            "maximum_frame_bytes": frame_length,
                        },
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
                "minimum_version": {"major": 1, "minor": 3},
                "maximum_version": {"major": 1, "minor": 3},
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
        self.daemon_instance_id = checked_instance(hello.get("daemon_instance_id"))
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
        if (
            checked_instance(snapshot.get("daemon_instance_id"))
            != self.daemon_instance_id
        ):
            raise RuntimeError(
                "daemon attach instance differs from negotiated identity"
            )
        return snapshot

    def close(self) -> None:
        try:
            self.send({"type": "detach"})
            self.receive(1)
        except (BrokenPipeError, EOFError, OSError, TimeoutError):
            pass
        self.socket.close()
