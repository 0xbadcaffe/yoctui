#!/usr/bin/env python3
"""Measure a real attach's first frame in a private-session PTY, without restarting its daemon."""

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time

from terminal_capture import Screen


def daemon_connected(text):
    # BitBake may be idle/disconnected while its persistent daemon is healthy.
    return any(marker in text for marker in (
        "D:✓ Connected/", "Daemon: ✓ Connected", "Daemon health: ✓ Connected/",
    ))


def watch_count(pid):
    try:
        return sum(
            line.startswith("inotify wd:")
            for path in Path(f"/proc/{pid}/fdinfo").iterdir()
            for line in path.read_text().splitlines()
        )
    except (FileNotFoundError, PermissionError, ProcessLookupError):
        return None


def measure(binary, timeout, probe_graphics=False):
    with tempfile.TemporaryDirectory(prefix="yoctui-attach-measure-") as directory:
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
        env = dict(os.environ, TERM="xterm-256color", YOCTUI_TERMINAL_GRAPHICS="none",
                   YOCTUI_GRAPHICS_TERMINAL_HANDOFF="1")
        if probe_graphics:
            env.pop("YOCTUI_TERMINAL_GRAPHICS", None)
        screen = Screen(160, 50)
        started = time.monotonic()
        process = subprocess.Popen(
            [str(binary), "--config", f"{directory}/config.toml", "attach"],
            stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True,
        )
        os.close(slave)
        first = None
        first_text = ""
        watches = None
        try:
            while time.monotonic() - started < timeout and process.poll() is None:
                if select.select([master], [], [], 0.05)[0]:
                    try:
                        screen.feed(os.read(master, 65536))
                    except OSError:
                        break
                    text = screen.text()
                    if first is None and "Navigator" in text:
                        # A real composed workspace, not alternate-screen setup.
                        if "Build Overview" in text or "Build environment" in text or "Build Environment" in text:
                            first = time.monotonic() - started
                            first_text = text
                if first is not None and time.monotonic() - started >= first + 2:
                    watches = watch_count(process.pid)
                    break
            text = first_text or screen.text()
            return {
                "binary": str(binary),
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "first_frame_seconds": round(first, 6) if first is not None else None,
                "inotify_watches_after_two_seconds": watches,
                "connected": daemon_connected(screen.text()),
                "workspace_header": [line for line in text.splitlines()[:5] if "yoctui v" in line or "Workspace:" in line],
                "dimensions": [160, 50],
                "terminal_graphics": "native probe (no desktop handoff)" if probe_graphics else "none (no desktop handoff/probe)",
                "session": "private temporary config/session; existing daemon untouched",
            }
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            os.close(master)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--timeout", type=float, default=60)
    parser.add_argument("--limit", type=float, default=8)
    parser.add_argument("--probe-graphics", action="store_true", help="Include the normal terminal graphics query, but do not open a desktop window")
    args = parser.parse_args()
    result = measure(args.binary.resolve(strict=True), args.timeout, args.probe_graphics)
    print(json.dumps(result, indent=2))
    return 0 if result["connected"] and result["first_frame_seconds"] is not None and result["first_frame_seconds"] <= args.limit else 1


if __name__ == "__main__":
    raise SystemExit(main())
