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
                "minimum_version": {"major": 1, "minor": 3},
                "maximum_version": {"major": 1, "minor": 3},
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

    def detach(self, timeout: float = 2.0) -> tuple[int, int]:
        sent_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
        self.send({"type": "detach"})
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            received = self.receive(max(0.001, deadline - time.monotonic()))
            if received is None:
                break
            message, received_ns = received
            if message.get("type") == "detaching":
                self.socket.close()
                return sent_ns, received_ns
        raise RuntimeError("daemon did not acknowledge detach")
