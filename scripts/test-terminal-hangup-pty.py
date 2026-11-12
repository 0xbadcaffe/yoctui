#!/usr/bin/env python3
"""An attached UI must exit after PTY EOF, with or without SIGHUP."""
import fcntl
import os
from pathlib import Path
import pty
import struct
import subprocess
import tempfile
import termios
import time

from pty_acceptance import (
    TerminalAcceptance,
    become_session_leader,
    binary_path,
    isolated_environment,
    snapshot_fixture,
)

ROOT = Path(__file__).resolve().parents[1]


def check_hangup(environment, build, controlling_terminal, fragment, onboarding):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
    try:
        process = subprocess.Popen(
            [binary_path(ROOT), "--no-color", "--build-dir", build, "attach"],
            stdin=slave, stdout=slave, stderr=subprocess.PIPE, env=environment,
            # Without TIOCSCTTY the kernel sends no SIGHUP: EOF detection must
            # work independently of the application's signal handler.
            preexec_fn=become_session_leader if controlling_terminal else os.setsid,
        )
    finally:
        os.close(slave)
    terminal = TerminalAcceptance(master, process, bytearray(), 100, 30)
    try:
        if onboarding:
            terminal.dismiss_onboarding()
        else:
            terminal.wait_for("Navigator")
        # Check ordinary key input before exercising EOF inside the reader.
        os.write(master, b"\x1b[20~")
        terminal.wait_for("Command")
        if fragment:
            os.write(master, fragment)
        time.sleep(0.02)
        os.close(master)
        master = None
        process.wait(timeout=5)
        errors = process.stderr.read().decode("utf-8", "replace")
        assert process.returncode == 0, f"hangup exit: {process.returncode}: {errors}"
    finally:
        terminal.cleanup()
        process.stderr.close()
        if master is not None:
            os.close(master)


def main():
    with tempfile.TemporaryDirectory(prefix="yoctui-hangup-") as directory:
        environment = isolated_environment(directory)
        build = snapshot_fixture(directory, ROOT, environment)
        prefix = [binary_path(ROOT), "--backend", "process", "--build-dir", build, "daemon"]
        subprocess.run(prefix + ["start"], env=environment, check=True, capture_output=True)
        try:
            onboarding = True
            for controlling_terminal in (True, False):
                for fragment in (b"", b"\x1b[", b"\x1b[200~unfinished paste"):
                    check_hangup(environment, build, controlling_terminal, fragment, onboarding)
                    onboarding = False
            # Attach clients must leave the build daemon available.
            subprocess.run(prefix + ["status"], env=environment, check=True, capture_output=True)
        finally:
            subprocess.run(prefix + ["stop"], env=environment, check=True, capture_output=True)
    print("PTY hangup regression passed (idle, partial escape, partial paste; with/without SIGHUP)")


if __name__ == "__main__":
    main()
