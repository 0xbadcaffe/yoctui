#!/usr/bin/env python3
"""Focused health-line regression checks for the real PTY capture helper."""
import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "live_capture", Path(__file__).with_name("capture-live-next-generation-ui.py")
)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)


class CaptureHealthTests(unittest.TestCase):
    def test_current_health_line(self):
        self.assertTrue(capture.daemon_connected("Daemon health: ✓ Connected/Local"))

    def test_historical_health_line(self):
        self.assertTrue(capture.daemon_connected("Daemon: ✓ Connected"))

    def test_disconnected(self):
        self.assertFalse(capture.daemon_connected("Daemon health: ✕ Disconnected/Local"))

    def test_abbreviated_header_is_not_health_evidence(self):
        self.assertFalse(capture.daemon_connected("D:✓ Connected/Local"))

    def test_contradictory_health_is_rejected(self):
        self.assertFalse(
            capture.daemon_connected("Daemon: ✓ Connected Daemon: ✕ Disconnected")
        )


if __name__ == "__main__":
    unittest.main()
