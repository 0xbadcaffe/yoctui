"""Readiness regressions; these fixtures never represent performance evidence."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "capture_real_poky", Path(__file__).with_name("capture-real-poky-performance.py")
)
assert SPEC and SPEC.loader
CAPTURE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CAPTURE)
VARIABLES = {"BB_NUMBER_THREADS": "8", "PARALLEL_MAKE": "-j 8"}
WORKSPACE = {"type": "workspace", "data": {"variables": VARIABLES}}


class Client:
    def __init__(self, messages=(), failure=None):
        self.messages = iter(messages)
        self.failure = failure
        self.sent = []
        self.reads = 0

    def receive(self, timeout):
        assert 0 < timeout <= 0.5
        self.reads += 1
        if self.failure:
            raise self.failure
        message = next(self.messages, None)
        return (message, 0) if message else None

    def send(self, message):
        assert message["type"] == "pong", "readiness must never submit a build"
        self.sent.append(message)


class InitialWorkspaceTests(unittest.TestCase):
    def snapshot(self, events=()):
        return {"daemon_instance_id": [1] * 16, "build_events": list(events)}

    def test_ready_snapshot_needs_no_read_and_delayed_workspace_is_awaited(self):
        client = Client()
        self.assertEqual(
            CAPTURE.wait_for_initial_workspace(client, self.snapshot([WORKSPACE])),
            VARIABLES,
        )
        self.assertEqual(client.reads, 0)
        client = Client(
            [
                None,
                {"type": "ping", "nonce": 9},
                {"type": "event", "event": {"type": "telemetry", "data": {}}},
                {"type": "event", "event": {"type": "build", "data": WORKSPACE}},
            ]
        )
        self.assertEqual(
            CAPTURE.wait_for_initial_workspace(client, self.snapshot()), VARIABLES
        )
        self.assertEqual(client.sent, [{"type": "pong", "nonce": 9}])

    def test_snapshot_replacement_keeps_instance_and_rejects_missing_variables(self):
        replacement = {"type": "snapshot", **self.snapshot([WORKSPACE])}
        self.assertEqual(
            CAPTURE.wait_for_initial_workspace(Client([replacement]), self.snapshot()),
            VARIABLES,
        )
        replacement["daemon_instance_id"] = [2] * 16
        with self.assertRaisesRegex(RuntimeError, "instance changed"):
            CAPTURE.wait_for_initial_workspace(Client([replacement]), self.snapshot())
        for variables in [{}, {"BB_NUMBER_THREADS": None, "PARALLEL_MAKE": "-j 8"}]:
            malformed = {"type": "workspace", "data": {"variables": variables}}
            with self.assertRaisesRegex(RuntimeError, "parallelism"):
                CAPTURE.wait_for_initial_workspace(Client(), self.snapshot([malformed]))

    def test_failure_disconnect_timeout_and_message_bound_do_not_submit_commands(self):
        error_log = {"message": "Initial metadata scan failed: fixture failure"}
        snapshot = self.snapshot()
        snapshot["recent_logs"] = [error_log]
        with self.assertRaisesRegex(RuntimeError, "fixture failure"):
            CAPTURE.wait_for_initial_workspace(Client(), snapshot)
        for message in [
            {"type": "event", "event": {"type": "log", "data": error_log}},
            {"type": "error", "message": "protocol failure"},
            {"type": "shutting_down"},
        ]:
            with self.assertRaises(RuntimeError):
                CAPTURE.wait_for_initial_workspace(Client([message]), self.snapshot())
        with self.assertRaises(EOFError):
            CAPTURE.wait_for_initial_workspace(
                Client(failure=EOFError("lost")), self.snapshot()
            )
        with patch.object(CAPTURE.time, "monotonic", side_effect=[0, 0, 1]):
            with self.assertRaisesRegex(RuntimeError, "timed out"):
                CAPTURE.wait_for_initial_workspace(
                    Client(), self.snapshot(), timeout=0.5
                )
        messages = (
            {"type": "event", "event": {"type": "telemetry"}} for _ in range(8192)
        )
        client = Client(messages)
        with patch.object(CAPTURE.time, "monotonic", return_value=0):
            with self.assertRaisesRegex(RuntimeError, "message bound"):
                CAPTURE.wait_for_initial_workspace(client, self.snapshot())
        self.assertEqual(client.reads, 8192)


if __name__ == "__main__":
    unittest.main()
