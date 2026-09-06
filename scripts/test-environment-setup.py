#!/usr/bin/env python3
"""Exercise real disconnected startup/path browsing in a private PTY; no BitBake."""

from __future__ import annotations

import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time

from terminal_capture import Screen


def main() -> None:
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/yoctui").resolve()
    with tempfile.TemporaryDirectory(prefix="yoctui-environment-pty-") as temporary:
        root = Path(temporary)
        source = root / "poky"
        build = root / "build"
        source.mkdir()
        build.mkdir()
        (source / "nested").mkdir()
        marker = root / "must-not-run"
        script = source / "oe-init-build-env"
        script.write_text(f"#!/bin/sh\ntouch '{marker}'\nexit 23\n")
        script.chmod(0o700)
        environment = os.environ.copy()
        for key in list(environment):
            if key.startswith("YOCTUI_") or key in {
                "BUILDDIR",
                "BBPATH",
                "BBSERVER",
                "TEMPLATECONF",
            }:
                environment.pop(key)
        for key in ("XDG_CONFIG_HOME", "XDG_STATE_HOME", "XDG_RUNTIME_DIR"):
            directory = root / key.lower()
            directory.mkdir(mode=0o700)
            environment[key] = str(directory)
        environment["TERM"] = "xterm-256color"
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 32, 120, 0, 0))
        process = subprocess.Popen(
            [str(binary), "--backend", "process", "--no-color"],
            stdin=slave,
            stdout=slave,
            stderr=slave,
            env=environment,
            cwd=root,
            start_new_session=True,
        )
        os.close(slave)
        screen = Screen(120, 32)

        def wait_for(needle: str, absent: str | None = None) -> None:
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                ready, _, _ = select.select([master], [], [], 0.05)
                if ready:
                    try:
                        screen.feed(os.read(master, 65536))
                    except OSError:
                        break
                text = screen.text()
                if needle in text and (absent is None or absent not in text):
                    return
                if process.poll() is not None:
                    break
            raise AssertionError(
                f"Missing {needle!r} / unexpected {absent!r}:\n{screen.text()}"
            )

        def send(value: bytes) -> None:
            os.write(master, value)

        def edit(value: Path) -> None:
            send(b"e")
            wait_for("Enter accept value")
            send(b"\x1b[200~" + str(value).encode() + b"\x1b[201~\r")
            wait_for("s save profile", "Enter accept value")

        try:
            wait_for("yoctui")
            send(b"\x1b")  # dismiss first-run onboarding or initial notice
            time.sleep(0.1)
            send(b"E")
            wait_for("Build environment")
            send(b"e")
            wait_for("Configure build environment")
            edit(source)
            send(b"b")
            wait_for("nested/", "Loading directories")
            send(b"\r")
            wait_for("No child directories", "Loading directories")
            send(b"\x1b[D")
            wait_for("nested/", "Loading directories")
            send(b"s")
            wait_for("oe-init-build-env", "Browse directories")
            send(b"\t")
            edit(build)
            send(b"b")
            wait_for("No child directories", "Loading directories")
            send(b"s")
            wait_for("Configure build environment", "Browse directories")
            send(b"s")
            wait_for("configured", "Configure build environment")
            assert not marker.exists(), "Browsing or saving executed the source script"
            assert not list((root / "xdg_runtime_dir").rglob("daemon.sock"))
            send(b"q")
            wait_for("Are you sure you want to exit yoctui?")
            send(b"y")
            process.wait(timeout=5)
            assert process.returncode == 0
            print(
                "Environment setup PTY passed: manual paste, source/build browse, hierarchy, save; no initialization or daemon"
            )
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            os.close(master)


if __name__ == "__main__":
    main()
