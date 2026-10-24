#!/usr/bin/env python3
"""Verify native UI exit restores mouse/input modes in isolated PTYs.

The metadata fixture is not a live image build. No existing daemon is stopped.
"""
import argparse
import json
import os
from pathlib import Path
import re
import signal
import tempfile
import termios
import time

from pty_acceptance import isolated_environment, snapshot_fixture, start_terminal

ROOT = Path(__file__).resolve().parents[1]
MOUSE_MODES = (1000, 1002, 1003, 1006, 1015)


def restored(terminal):
    modes = {int(key): value == b"h" for key, value in re.findall(
        rb"\x1b\[\?(\d+)([hl])", bytes(terminal.raw))}
    for mode in (*MOUSE_MODES, 1049, 2004):
        assert modes.get(mode) is False, f"terminal mode {mode} was not disabled: {modes}"
    assert modes.get(25) is True, "cursor was not restored"
    flags = termios.tcgetattr(terminal.master)[3]
    assert flags & termios.ICANON and flags & termios.ECHO, "raw input mode was not restored"
    return modes


def exercise(name, sig, early, captures, queued_reports=0):
    with tempfile.TemporaryDirectory(prefix="yoctui-terminal-shutdown-") as tmp:
        environment = isolated_environment(tmp)
        build = snapshot_fixture(tmp, ROOT, environment)
        terminal = start_terminal(ROOT, environment, 100, 30,
                                  "--backend", "process", "--build-dir", build)
        observer = None
        try:
            if early:
                deadline = time.monotonic() + 8
                while b"\x1b[?1049h" not in terminal.raw:
                    assert time.monotonic() < deadline, "terminal did not enter its UI"
                    terminal.read()
            else:
                terminal.dismiss_onboarding()
            if queued_reports:
                # Keep a slave reader as a stand-in for Bash, with no shell
                # executing the injected terminal protocol bytes.
                observer = os.open(os.readlink(f"/proc/{terminal.process.pid}/fd/0"),
                                   os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
                os.write(terminal.master, b"q")
                terminal.wait_for("Are you sure you want to exit yoctui?")
                os.write(terminal.master, b"y" + b"\x1b[<35;66;20M" * queued_reports)
            elif sig is None:
                terminal.finish()
            else:
                terminal.process.send_signal(sig)
            if sig is not None or queued_reports:
                deadline = time.monotonic() + 8
                while terminal.process.poll() is None and time.monotonic() < deadline:
                    terminal.read()
                assert terminal.process.poll() is not None, f"{name} did not shut down"
                while terminal.read(.05):
                    pass
            assert terminal.process.returncode == 0, f"{name}: exit {terminal.process.returncode}"
            modes = restored(terminal)
            if observer is not None:
                # Canonical Bash input becomes readable on Enter. The shell
                # must receive only that newline, without queued mouse bytes.
                os.write(terminal.master, b"\n")
                deadline = time.monotonic() + 2
                pending = b""
                while time.monotonic() < deadline:
                    try:
                        pending = os.read(observer, 65536)
                        break
                    except BlockingIOError:
                        terminal.read(.02)
                assert pending == b"\n", f"queued input leaked to shell: {pending[:100]!r}"
            assert (Path(environment["XDG_CONFIG_HOME"]) / "yoctui/session.toml").is_file(), \
                "graceful exit did not persist the client session"
            return {"case": name, "exit": terminal.process.returncode, "modes": modes}
        finally:
            captures.mkdir(parents=True, exist_ok=True)
            (captures / f"{name}.ansi").write_bytes(terminal.raw)
            terminal.cleanup()
            os.close(terminal.master)
            if observer is not None:
                os.close(observer)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture-dir", type=Path, default=ROOT / "artifacts/terminal-shutdown")
    args = parser.parse_args()
    cases = [("quit", None, False), ("sigterm", signal.SIGTERM, False),
             ("sighup", signal.SIGHUP, False), ("sigint", signal.SIGINT, False),
             ("startup-sighup", signal.SIGHUP, True), ("startup-sigint", signal.SIGINT, True)]
    results = []
    for name, sig, early in cases:
        results.append(exercise(name, sig, early, args.capture_dir))
        print(f"PASS {name}: mouse tracking, raw mode, cursor, alternate screen, session save", flush=True)
    for count in (100, 250):
        name = f"queued-mouse-{count}"
        results.append(exercise(name, None, False, args.capture_dir, queued_reports=count))
        print(f"PASS {name}: parent shell receives no queued mouse reports", flush=True)
    (args.capture_dir / "results.json").write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
