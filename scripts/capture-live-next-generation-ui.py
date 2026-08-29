#!/usr/bin/env python3
"""Capture one real Yoctui PTY frame as raw ANSI plus a semantic cell buffer."""

from __future__ import annotations

import argparse
import fcntl
import os
import pty
import re
import select
import signal
import struct
import subprocess
import termios
import time
from pathlib import Path

from terminal_capture import Screen as StyledScreen


CSI = re.compile(r"\x1b\[([0-9;?]*)([ -/]*)([@-~])")


class Screen:
    def __init__(self, width: int, height: int) -> None:
        self.width = width
        self.height = height
        self.cells = [[" " for _ in range(width)] for _ in range(height)]
        self.x = 0
        self.y = 0

    def clear(self) -> None:
        self.cells = [[" " for _ in range(self.width)] for _ in range(self.height)]
        self.x = 0
        self.y = 0

    def put(self, character: str) -> None:
        if 0 <= self.y < self.height and 0 <= self.x < self.width:
            self.cells[self.y][self.x] = character
        self.x += 1
        if self.x >= self.width:
            self.x = 0
            self.y = min(self.height - 1, self.y + 1)

    def csi(self, params: str, command: str) -> None:
        values = [int(value) for value in params.lstrip("?").split(";") if value]
        amount = values[0] if values else 1
        if command in ("H", "f"):
            self.y = max(0, min(self.height - 1, (values[0] if values else 1) - 1))
            self.x = max(
                0, min(self.width - 1, (values[1] if len(values) > 1 else 1) - 1)
            )
        elif command == "A":
            self.y = max(0, self.y - amount)
        elif command == "B":
            self.y = min(self.height - 1, self.y + amount)
        elif command == "C":
            self.x = min(self.width - 1, self.x + amount)
        elif command == "D":
            self.x = max(0, self.x - amount)
        elif command == "G":
            self.x = max(0, min(self.width - 1, amount - 1))
        elif command == "d":
            self.y = max(0, min(self.height - 1, amount - 1))
        elif command == "J" and params.lstrip("?") in ("2", "3"):
            self.clear()
        elif command == "K":
            for column in range(self.x, self.width):
                self.cells[self.y][column] = " "

    def feed(self, text: str) -> None:
        index = 0
        while index < len(text):
            if text[index] == "\x1b":
                match = CSI.match(text, index)
                if match:
                    self.csi(match.group(1), match.group(3))
                    index = match.end()
                    continue
                if index + 1 < len(text):
                    index += 2
                    continue
            character = text[index]
            if character == "\r":
                self.x = 0
            elif character == "\n":
                self.y = min(self.height - 1, self.y + 1)
            elif character == "\b":
                self.x = max(0, self.x - 1)
            elif character >= " ":
                self.put(character)
            index += 1

    def text(self) -> str:
        return "\n".join("".join(row).rstrip() for row in self.cells).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--build-dir", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument(
        "--mode", choices=("tasks", "terminal", "dashboard"), required=True
    )
    parser.add_argument("--backend", choices=("bridge", "process"), default="bridge")
    parser.add_argument("--seconds", type=float, default=6.0)
    parser.add_argument("--startup-seconds", type=float, default=30.0)
    parser.add_argument("--expect", action="append", default=[])
    parser.add_argument("--ready-file")
    parser.add_argument("--width", type=int, default=160)
    parser.add_argument("--height", type=int, default=50)
    args = parser.parse_args()

    master, slave = pty.openpty()
    fcntl.ioctl(
        slave,
        termios.TIOCSWINSZ,
        struct.pack("HHHH", args.height, args.width, 0, 0),
    )

    def become_session_leader() -> None:
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    process = subprocess.Popen(
        [args.binary, "--backend", args.backend, "--build-dir", args.build_dir],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=os.environ.copy(),
        preexec_fn=become_session_leader,
    )
    os.close(slave)
    raw = bytearray()

    def collect(seconds: float) -> None:
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline and process.poll() is None:
            ready, _, _ = select.select([master], [], [], 0.1)
            if not ready:
                continue
            try:
                chunk = os.read(master, 65536)
            except OSError:
                break
            raw.extend(chunk)

    startup_deadline = time.monotonic() + args.startup_seconds
    while time.monotonic() < startup_deadline and process.poll() is None:
        collect(0.5)
        startup_screen = StyledScreen(args.width, args.height)
        startup_screen.feed(bytes(raw).decode("utf-8", "replace"))
        if "Daemon: ✓ Connected" in startup_screen.text():
            break
    if args.mode == "tasks":
        os.write(master, b"\x1bOQ")
    elif args.mode == "terminal":
        os.write(master, b"\x1b")
        collect(0.4)
        os.write(master, b"\x02c")
    else:
        os.write(master, b"\x1b")
    if args.ready_file:
        ready_file = Path(args.ready_file)
        ready_file.parent.mkdir(parents=True, exist_ok=True)
        ready_file.write_text("connected\n", encoding="utf-8")
    if args.expect:
        expected_deadline = time.monotonic() + args.seconds
        while time.monotonic() < expected_deadline and process.poll() is None:
            collect(0.5)
            expected_screen = StyledScreen(args.width, args.height)
            expected_screen.feed(bytes(raw).decode("utf-8", "replace"))
            if all(value in expected_screen.text() for value in args.expect):
                break
    else:
        collect(args.seconds)

    screen = StyledScreen(args.width, args.height)
    screen.feed(bytes(raw).decode("utf-8", "replace"))

    final_text = screen.text()
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.with_suffix(".ansi").write_bytes(bytes(raw[-2_000_000:]))
    output.with_suffix(".txt").write_text(final_text, encoding="utf-8")
    output.with_suffix(".cells").write_text(screen.cell_golden(), encoding="utf-8")
    output.with_suffix(".meta").write_text(
        f"label=live\nwidth={args.width}\nheight={args.height}\n"
        f"mode={args.mode}\nraw_bytes={min(len(raw), 2_000_000)}\n",
        encoding="utf-8",
    )

    os.write(master, b"q\r")
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=2)
    os.close(master)
    if b"\x1b[?1049h" not in raw:
        raise SystemExit("Yoctui did not enter the alternate screen")
    if "F1 Help" not in final_text:
        raise SystemExit(f"final buffer omitted footer:\n{final_text}")
    if "Daemon: ✓ Connected" not in final_text:
        raise SystemExit(f"final buffer omitted connected daemon state:\n{final_text}")
    for expected in args.expect:
        if expected not in final_text:
            raise SystemExit(f"final buffer omitted expected {expected!r}:\n{final_text}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
