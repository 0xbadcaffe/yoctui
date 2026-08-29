#!/usr/bin/env python3
"""Drive the non-trivial M22 workflows through one real Yoctui PTY client."""

from __future__ import annotations

import argparse
import codecs
import fcntl
import hashlib
import json
import os
import pty
import re
import select
import signal
import struct
import subprocess
import termios
import time
import unicodedata
from pathlib import Path

from terminal_capture import Screen as StyledScreen


CSI = re.compile(r"\x1b\[([0-9;?]*)([ -/]*)?([@-~])")


class Screen:
    """Compose a cursor-addressed crossterm stream into semantic cells."""

    def __init__(self, rows: int, columns: int) -> None:
        self.rows = rows
        self.columns = columns
        self.cells = [[" "] * columns for _ in range(rows)]
        self.row = 0
        self.column = 0
        self.saved = (0, 0)
        self.pending = ""
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def text(self) -> str:
        return "\n".join("".join(row).rstrip() for row in self.cells).rstrip() + "\n"

    def feed(self, raw: bytes) -> None:
        self.pending += self.decoder.decode(raw)
        index = 0
        while index < len(self.pending):
            character = self.pending[index]
            if character != "\x1b":
                self.write(character)
                index += 1
                continue
            if index + 1 >= len(self.pending):
                break
            kind = self.pending[index + 1]
            if kind == "[":
                match = CSI.match(self.pending, index)
                if match is None:
                    break
                self.csi(match.group(1), match.group(3))
                index = match.end()
                continue
            if kind == "]":
                bell = self.pending.find("\x07", index + 2)
                string_term = self.pending.find("\x1b\\", index + 2)
                endings = [value for value in (bell, string_term) if value >= 0]
                if not endings:
                    break
                end = min(endings)
                index = end + (2 if self.pending[end : end + 2] == "\x1b\\" else 1)
                continue
            if kind in "()":
                if index + 2 >= len(self.pending):
                    break
                index += 3
                continue
            index += 2
        self.pending = self.pending[index:]

    def write(self, character: str) -> None:
        if character == "\r":
            self.column = 0
        elif character == "\n":
            self.row = min(self.rows - 1, self.row + 1)
        elif character == "\b":
            self.column = max(0, self.column - 1)
        elif character == "\t":
            self.column = min(self.columns - 1, (self.column // 8 + 1) * 8)
        elif ord(character) >= 0x20 and character != "\x7f":
            width = 2 if unicodedata.east_asian_width(character) in ("W", "F") else 1
            if unicodedata.combining(character):
                width = 0
            if self.row < self.rows and self.column < self.columns:
                self.cells[self.row][self.column] = character
            self.column = min(self.columns - 1, self.column + width)

    def csi(self, parameters: str, command: str) -> None:
        values = [int(value) if value else 0 for value in parameters.lstrip("?").split(";")]
        first = values[0] if values else 0
        amount = first or 1
        if command in ("H", "f"):
            self.row = min(self.rows - 1, max(0, (values[0] if values else 1) - 1))
            self.column = min(
                self.columns - 1,
                max(0, (values[1] if len(values) > 1 else 1) - 1),
            )
        elif command == "A":
            self.row = max(0, self.row - amount)
        elif command == "B":
            self.row = min(self.rows - 1, self.row + amount)
        elif command == "C":
            self.column = min(self.columns - 1, self.column + amount)
        elif command == "D":
            self.column = max(0, self.column - amount)
        elif command == "E":
            self.row = min(self.rows - 1, self.row + amount)
            self.column = 0
        elif command == "F":
            self.row = max(0, self.row - amount)
            self.column = 0
        elif command in ("G", "`"):
            self.column = min(self.columns - 1, max(0, amount - 1))
        elif command == "d":
            self.row = min(self.rows - 1, max(0, amount - 1))
        elif command == "J":
            if first in (2, 3):
                self.cells = [[" "] * self.columns for _ in range(self.rows)]
            elif first == 0:
                self.cells[self.row][self.column :] = [" "] * (self.columns - self.column)
                for row in range(self.row + 1, self.rows):
                    self.cells[row] = [" "] * self.columns
            elif first == 1:
                for row in range(self.row):
                    self.cells[row] = [" "] * self.columns
                self.cells[self.row][: self.column + 1] = [" "] * (self.column + 1)
        elif command == "K":
            if first == 0:
                self.cells[self.row][self.column :] = [" "] * (self.columns - self.column)
            elif first == 1:
                self.cells[self.row][: self.column + 1] = [" "] * (self.column + 1)
            elif first == 2:
                self.cells[self.row] = [" "] * self.columns
        elif command == "s":
            self.saved = (self.row, self.column)
        elif command == "u":
            self.row, self.column = self.saved


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--build-dir", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--scenario", choices=("errors", "rootfs", "editor-menu"), required=True)
    parser.add_argument("--backend", choices=("bridge", "process"), default="process")
    parser.add_argument("--width", type=int, default=160)
    parser.add_argument("--height", type=int, default=50)
    args = parser.parse_args()

    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", args.height, args.width, 0, 0))

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
    screen = StyledScreen(args.width, args.height)
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
            screen.feed(chunk)

    def expect(anchors: list[str], label: str, seconds: float) -> str:
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline and process.poll() is None:
            collect(0.25)
            visible = screen.text()
            if all(anchor in visible for anchor in anchors):
                return visible
        raise SystemExit(
            f"{label} omitted {anchors!r} (returncode={process.poll()}):\n{screen.text()}"
        )

    expect(["yoctui", "Daemon: ✓ Connected"], "live startup", 20)
    interactions: list[str]
    assertions: list[str]
    if args.scenario == "errors":
        interactions = ["press e to open the Errors workspace after an intentional failed build"]
        assertions = ["Errors", "Failed", "Daemon: ✓ Connected"]
        os.write(master, b"e")
        expect(assertions, "failed-build Errors workflow", 20)
    elif args.scenario == "rootfs":
        interactions = [
            "press F8 to open authoritative image artifacts",
            "wait for the DEPLOY_DIR_IMAGE scan to select the built artifact",
            "filter the artifact inventory to the real ext4 root filesystem",
            "press p to inspect the selected image rootfs composition",
        ]
        os.write(master, b"\x1b[19~")
        assertions = [
            "Images · Rootfs composition",
            "Installed-package authority",
            "Exact bytes",
            "Installed packages:",
            "Exact totals",
        ]
        deadline = time.monotonic() + 180
        next_retry = 0.0
        selected_rootfs = False
        while time.monotonic() < deadline and process.poll() is None:
            collect(0.25)
            visible = screen.text()
            if all(anchor in visible for anchor in assertions):
                break
            now = time.monotonic()
            if (
                "Images" in visible
                and "Loading deployed image artifacts" not in visible
                and now >= next_retry
            ):
                if not selected_rootfs:
                    os.write(master, b"/ext4\r")
                    selected_rootfs = True
                else:
                    os.write(master, b"p")
                next_retry = now + 2
        else:
            raise SystemExit(
                f"rootfs composition workflow omitted {assertions!r} "
                f"(returncode={process.poll()}):\n{screen.text()}"
            )
    else:
        interactions = [
            "press F7 to open the daemon-authoritative Recipes inventory",
            "type /busybox and Enter, then select the exact busybox recipe from bounded matches",
            "press t to refresh authoritative Devtool status",
            "press d and Enter to run confirmed devtool modify",
            "press F10 to compose the application menu over the recipe editor",
        ]
        os.write(master, b"\x1b[18~")
        expect(
            ["Recipes (shown:", "Provider file:"],
            "daemon-authoritative recipe inventory",
            60,
        )
        os.write(master, b"/busybox\r")
        expect(["Query: busybox", "Recipes (shown: 3"], "busybox recipe filter", 10)
        for _ in range(3):
            collect(0.25)
            if "Recipe: busybox " in screen.text():
                break
            os.write(master, b"\x1b[B")
        else:
            raise SystemExit(
                "exact busybox recipe selection was absent from three bounded matches:\n"
                + screen.text()
            )
        os.write(master, b"t")
        expect(["Workspace/Devtool: not in workspace"], "authoritative Devtool status", 30)
        os.write(master, b"d")
        expect(["Confirm Devtool modify"], "devtool modify confirmation", 10)
        os.write(master, b"\r")
        expect(["Recipe editor:", "Workspace file tree:"], "real devtool recipe editor", 600)
        os.write(master, b"\x1b[21~")
        assertions = ["Recipe editor:", "Application menu", "focus trapped"]
        expect(assertions, "application menu over recipe editor", 20)

    if "�" in screen.text():
        raise SystemExit("live screen contains a replacement glyph")
    scenario_id = {
        "errors": "failed-build-errors",
        "rootfs": "rootfs-composition",
        "editor-menu": "editor-application-menu",
    }[args.scenario]
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.with_suffix(".ansi").write_bytes(bytes(raw[-2_000_000:]))
    output.with_suffix(".txt").write_text(screen.text(), encoding="utf-8")
    output.with_suffix(".cells").write_text(screen.cell_golden(), encoding="utf-8")
    output.with_suffix(".meta").write_text(
        f"label=live\nscenario={scenario_id}\nwidth={args.width}\nheight={args.height}\n"
        f"raw_bytes={min(len(raw), 2_000_000)}\n",
        encoding="utf-8",
    )
    report = {
        "schema": 1,
        "scenario": scenario_id,
        "interactions": interactions,
        "observed_assertions": assertions,
        "terminal": output.with_suffix(".ansi").name,
        "semantic": output.with_suffix(".txt").name,
    }
    output.with_suffix(".report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=2)
    os.close(master)
    if b"\x1b[?1049h" not in raw:
        raise SystemExit("Yoctui did not enter the alternate screen")
    for suffix in (".ansi", ".txt", ".meta", ".report.json"):
        path = output.with_suffix(suffix)
        if not path.is_file() or not sha256(path):
            raise SystemExit(f"missing capture artifact: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
