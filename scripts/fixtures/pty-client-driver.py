#!/usr/bin/env python3
"""Run a command in a fixed-size PTY and inject timestamped byte sequences."""

from __future__ import annotations

import argparse
import fcntl
import os
import pty
import selectors
import signal
import struct
import subprocess
import termios
import time
from pathlib import Path


def scheduled_write(value: str) -> tuple[float, bytes]:
    try:
        raw_delay, escaped = value.split("=", 1)
        delay = float(raw_delay)
    except ValueError as error:
        raise argparse.ArgumentTypeError("write must be SECONDS=ESCAPED_BYTES") from error
    if delay < 0:
        raise argparse.ArgumentTypeError("write delay must be nonnegative")
    return delay, bytes(escaped, "utf-8").decode("unicode_escape").encode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pid-file", required=True, type=Path)
    parser.add_argument("--transcript", required=True, type=Path)
    parser.add_argument("--columns", type=int, default=160)
    parser.add_argument("--rows", type=int, default=50)
    parser.add_argument("--duration", type=float, default=120.0)
    parser.add_argument("--write", action="append", default=[], type=scheduled_write)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        parser.error("a command is required after --")
    if args.command[0] == "--":
        args.command = args.command[1:]
    if not args.command:
        parser.error("a command is required after --")

    master, slave = pty.openpty()
    fcntl.ioctl(
        slave,
        termios.TIOCSWINSZ,
        struct.pack("HHHH", args.rows, args.columns, 0, 0),
    )
    process = subprocess.Popen(
        args.command,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        start_new_session=True,
        close_fds=True,
    )
    os.close(slave)
    args.pid_file.write_text(f"{process.pid}\n", encoding="utf-8")
    selector = selectors.DefaultSelector()
    selector.register(master, selectors.EVENT_READ)
    started = time.monotonic()
    writes = iter(sorted(args.write))
    pending = next(writes, None)
    deadline = started + args.duration
    with args.transcript.open("wb") as transcript:
        try:
            while process.poll() is None and time.monotonic() < deadline:
                now = time.monotonic()
                while pending is not None and now - started >= pending[0]:
                    os.write(master, pending[1])
                    pending = next(writes, None)
                timeout = min(0.1, max(0.0, deadline - now))
                if pending is not None:
                    timeout = min(timeout, max(0.0, started + pending[0] - now))
                for _, _ in selector.select(timeout):
                    try:
                        data = os.read(master, 65536)
                    except OSError:
                        data = b""
                    if data:
                        transcript.write(data)
                        transcript.flush()
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
            os.close(master)
    return process.returncode or 0


if __name__ == "__main__":
    raise SystemExit(main())
