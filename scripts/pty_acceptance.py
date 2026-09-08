"""Bounded screen synchronization for the real terminal acceptance scripts."""

import atexit
import os
import select
import time

from terminal_capture import Screen


class TerminalAcceptance:
    def __init__(self, master, process, raw, width, height):
        self.master = master
        self.process = process
        self.raw = raw
        self.screen = Screen(width, height)
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
        self.screen.feed(data)
        return bool(data)

    def wait_for(self, needle, absent=None):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            self.read()
            text = self.screen.text()
            if needle in text and (absent is None or absent not in text):
                return
            if self.process.poll() is not None:
                break
        raise AssertionError(f"PTY missing {needle!r}:\n{self.screen.text()}")

    def dismiss_onboarding(self):
        # The OSC window title precedes the first frame and is not readiness.
        self.wait_for("Esc dismiss")
        os.write(self.master, b"\x1b")
        self.wait_for("Quit", absent="Esc dismiss")

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
