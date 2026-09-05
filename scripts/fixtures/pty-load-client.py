#!/usr/bin/env python3
"""Create a deterministic daemon PTY workload through the production IPC protocol."""

from __future__ import annotations

import argparse
import json
import os
import socket
import struct
import time
from pathlib import Path


def send(stream: socket.socket, message: dict[str, object]) -> None:
    payload = json.dumps(message, separators=(",", ":")).encode()
    stream.sendall(struct.pack(">I", len(payload)) + payload)


def receive(stream: socket.socket) -> dict[str, object]:
    prefix = b""
    while len(prefix) < 4:
        chunk = stream.recv(4 - len(prefix))
        if not chunk:
            raise RuntimeError("daemon disconnected while reading a frame prefix")
        prefix += chunk
    length = struct.unpack(">I", prefix)[0]
    payload = b""
    while len(payload) < length:
        chunk = stream.recv(length - len(payload))
        if not chunk:
            raise RuntimeError("daemon disconnected while reading a frame payload")
        payload += chunk
    value = json.loads(payload)
    if not isinstance(value, dict):
        raise RuntimeError("daemon frame is not a JSON object")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--socket", required=True, type=Path)
    parser.add_argument("--cwd", required=True, type=Path)
    parser.add_argument("--pid-file", required=True, type=Path)
    parser.add_argument("--duration", type=float, default=180.0)
    parser.add_argument("--lines-per-second", type=int, default=100)
    args = parser.parse_args()
    args.pid_file.write_text(f"{os.getpid()}\n", encoding="utf-8")
    delay = 1.0 / args.lines_per_second
    script = (
        "n=0; while :; do printf 'pty-active-%s\\n' \"$n\"; "
        f"n=$((n+1)); sleep {delay:.6f}; done"
    )
    stream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    stream.settimeout(10)
    stream.connect(str(args.socket))
    client_id = [71] * 16
    send(
        stream,
        {
            "type": "hello",
            "minimum_version": {"major": 1, "minor": 2},
            "maximum_version": {"major": 1, "minor": 2},
            "client_id": client_id,
            "client_name": "yoctui-pty-performance-fixture",
            "capabilities": [
                "state_snapshots",
                "incremental_events",
                "pty_sessions",
                "pty_writer_lease",
            ],
        },
    )
    hello = receive(stream)
    if hello.get("type") != "hello":
        raise RuntimeError(f"unexpected daemon hello response: {hello.get('type')}")
    send(
        stream,
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
        },
    )
    attached = receive(stream)
    if attached.get("type") != "attached":
        raise RuntimeError(f"unexpected daemon attach response: {attached.get('type')}")
    generation = attached["snapshot"]["generation"]
    send(
        stream,
        {
            "type": "command",
            "request_id": 1,
            "expected_generation": generation,
            "command": {
                "type": "create_pty",
                "name": "performance PTY output",
                "kind": "utility",
                "cwd": str(args.cwd.resolve(strict=True)),
                "command": {
                    "program": "/bin/sh",
                    "arguments": ["-c", script],
                    "environment_profile_id": None,
                },
                "dimensions": {"columns": 160, "rows": 50},
            },
        },
    )
    accepted = False
    deadline = time.monotonic() + args.duration
    stream.settimeout(1)
    while time.monotonic() < deadline:
        try:
            message = receive(stream)
        except TimeoutError:
            continue
        if message.get("type") == "command_result" and message.get("request_id") == 1:
            outcome = message.get("outcome", {})
            if outcome.get("type") != "accepted":
                raise RuntimeError(f"daemon rejected PTY workload: {outcome}")
            accepted = True
        elif message.get("type") == "ping":
            send(stream, {"type": "pong", "nonce": message["nonce"]})
    if not accepted:
        raise RuntimeError("daemon never accepted the PTY workload")
    send(stream, {"type": "detach"})
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
