"""Bounded screen synchronization for the real terminal acceptance scripts."""

import atexit
import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import sys
import termios
import time

from terminal_capture import Screen


def binary_path(root, environment=None):
    environment = os.environ if environment is None else environment
    target = Path(environment.get("CARGO_TARGET_DIR", "target")).expanduser()
    if not target.is_absolute():
        target = Path(root) / target
    binary = target / "debug" / "yoctui"
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise AssertionError(f"built acceptance binary missing: {binary}")
    return str(binary)


def isolated_environment(directory, environment=None):
    environment = os.environ if environment is None else environment
    isolated = {
        key: value for key, value in environment.items()
        if not key.startswith("YOCTUI_") and key not in {
            "BUILDDIR", "BBPATH", "BBSERVER", "TEMPLATECONF", "OEROOT",
            "BITBAKEDIR", "PYTHON", "PYTHONPATH", "PYTHONHOME",
        }
    }
    isolated["TERM"] = "xterm-256color"
    # Keep the real PTY under observation even on a graphical desktop.
    isolated["YOCTUI_TERMINAL_GRAPHICS"] = "none"
    for variable, name in (("XDG_CONFIG_HOME", "config"),
                           ("XDG_STATE_HOME", "state"),
                           ("XDG_RUNTIME_DIR", "runtime")):
        path = Path(directory) / name
        path.mkdir(mode=0o700)
        isolated[variable] = str(path)
    return isolated


def snapshot_fixture(directory, root, environment):
    """Initialized offline fixture, not live Yocto/BitBake compatibility proof."""
    directory = Path(directory)
    build = directory / "build"
    (build / "conf").mkdir(parents=True)
    (build / "conf/local.conf").write_text('MACHINE = "qemux86-64"\n')
    (build / "conf/bblayers.conf").write_text('BBLAYERS = ""\n')
    initializer = directory / "oe-init-build-env"
    initializer.write_text('export BUILDDIR="$1"\ncd "$BUILDDIR"\n')
    initializer.chmod(0o700)
    bin_dir = directory / "bin"
    bin_dir.mkdir()
    bitbake = bin_dir / "bitbake"
    bitbake.write_text('#!/bin/sh\n[ "$1" = --version ] || exit 1\necho "BitBake Build Tool Core version 2.18.0"\n')
    bitbake.chmod(0o700)
    environment["PATH"] = str(bin_dir) + os.pathsep + environment.get("PATH", "")
    environment["PYTHON"] = sys.executable
    environment["YOCTUI_BRIDGE_PATH"] = str(Path(root) / "scripts/fixtures/bitbake-ipc-latency-bridge.py")
    return str(build)


def become_session_leader():
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


def start_terminal(root, environment, width, height, *arguments):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
    try:
        process = subprocess.Popen(
            [binary_path(root), *arguments], stdin=slave, stdout=slave, stderr=slave,
            preexec_fn=become_session_leader, env=environment,
        )
    except BaseException:
        os.close(master)
        raise
    finally:
        os.close(slave)
    return TerminalAcceptance(master, process, bytearray(), width, height)


class TerminalAcceptance:
    def __init__(self, master, process, raw, width, height):
        self.master = master
        self.process = process
        self.raw = raw
        self.screen = Screen(width, height)
        self.cursor_query_pending = b""
        atexit.register(self.cleanup)

    def cleanup(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=2)

    def read(self, timeout=0.05):
        ready, _, _ = select.select([self.master], [], [], timeout)
        if not ready:
            return False
        try:
            data = os.read(self.master, 65536)
        except OSError:
            return False
        self.raw.extend(data)
        self.feed_output(data)
        return bool(data)

    def feed_output(self, data):
        # A PTY is not an emulator: answer CPR using the composed cursor, not
        # invented app state. Keep split requests bounded to three bytes.
        query = b"\x1b[6n"
        remaining = self.cursor_query_pending + data
        self.cursor_query_pending = b""
        while query in remaining:
            prefix, remaining = remaining.split(query, 1)
            self.screen.feed(prefix)
            reply = f"\x1b[{self.screen.y + 1};{self.screen.x + 1}R".encode()
            os.write(self.master, reply)
        for length in (3, 2, 1):
            if remaining.endswith(query[:length]):
                self.cursor_query_pending = remaining[-length:]
                remaining = remaining[:-length]
                break
        self.screen.feed(remaining)

    def wait_for(self, needle, absent=None, timeout=8):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            received = self.read()
            text = self.screen.text()
            # Drain the completed frame and its cursor queries before sending
            # keys; otherwise a partial footer can merge Escape with the CPR.
            if not received and needle in text and (absent is None or absent not in text):
                return
            if self.process.poll() is not None:
                break
        raise AssertionError(f"PTY missing {needle!r}:\n{self.screen.text()}")

    def resize(self, width, height):
        self.screen = Screen(width, height)
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))

    def unwind(self):
        for _ in range(4):
            os.write(self.master, b"\x1b")
            deadline = time.monotonic() + .15
            while time.monotonic() < deadline:
                self.read()

    def dismiss_onboarding(self, workbench_timeout=8):
        # The OSC window title precedes the first frame and is not readiness.
        self.wait_for("Esc dismiss")
        os.write(self.master, b"\x1b")
        self.wait_for("Quit", absent="Esc dismiss", timeout=workbench_timeout)

    def finish(self):
        os.write(self.master, b"q")
        deadline = time.monotonic() + 3
        confirmed = False
        while time.monotonic() < deadline:
            # A blocked PTY write can prevent the client from handling input.
            self.read()
            if not confirmed and "Are you sure you want to exit yoctui?" in self.screen.text():
                os.write(self.master, b"y")
                confirmed = True
            if self.process.poll() is not None:
                break
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait(timeout=2)
        while self.read(timeout=0.1):
            pass
        if self.process.returncode != 0:
            tail = bytes(self.raw[-4096:]).decode("utf-8", "replace")
            raise AssertionError(f"PTY did not exit cleanly: {self.process.returncode}\n{tail}")
        if b"\x1b[?1049h" not in self.raw or b"\x1b[?1049l" not in self.raw:
            raise AssertionError("PTY alternate screen was not entered and restored")
