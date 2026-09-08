#!/usr/bin/env python3
"""Regression tests for the deterministic BitBake-like event flood fixture."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import Mock, patch


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "scripts/fixtures/bitbake-event-flood-bridge.py"
HARNESS_PATH = ROOT / "scripts/event-flood-harness.py"
HARNESS_SPEC = importlib.util.spec_from_file_location(
    "event_flood_harness", HARNESS_PATH
)
assert HARNESS_SPEC and HARNESS_SPEC.loader
HARNESS = importlib.util.module_from_spec(HARNESS_SPEC)
HARNESS_SPEC.loader.exec_module(HARNESS)


class BridgeSession:
    def __init__(self, environment: dict[str, str]) -> None:
        self.process = subprocess.Popen(
            [str(FIXTURE)],
            cwd=environment["YOCTUI_TEST_BUILD_DIR"],
            env=environment,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.sequence = 0
        self.pending = bytearray()
        assert self.process.stdout is not None
        os.set_blocking(self.process.stdout.fileno(), False)

    def command(self, kind: str) -> None:
        assert self.process.stdin is not None
        self.sequence += 1
        envelope = {
            "protocol_version": 1,
            "sequence": self.sequence,
            "correlation_id": f"test-{self.sequence}",
            "message": {"type": kind},
        }
        self.process.stdin.write((json.dumps(envelope) + "\n").encode())
        self.process.stdin.flush()

    def event(self, timeout: float = 3) -> dict[str, object]:
        assert self.process.stdout is not None
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if b"\n" in self.pending:
                line, _, remainder = self.pending.partition(b"\n")
                self.pending = bytearray(remainder)
                return json.loads(line)["message"]
            try:
                chunk = os.read(self.process.stdout.fileno(), 65_536)
            except BlockingIOError:
                chunk = None
            if chunk:
                self.pending.extend(chunk)
                continue
            if chunk == b"" and self.process.poll() is not None:
                raise EOFError
            time.sleep(0.002)
        raise AssertionError("fixture did not emit a bounded event")

    def stop(self) -> None:
        if self.process.poll() is None:
            self.command("shutdown")
            try:
                self.event()
            except EOFError:
                pass
        self.process.communicate(timeout=3)
        for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
            if stream is not None:
                stream.close()


class EventFloodReadinessTests(unittest.TestCase):
    instance = list(range(16))
    build_dir = Path("/fixture/build")

    def snapshot(self, ready=False):
        return {
            "daemon_instance_id": self.instance.copy(),
            "sequence": 3,
            "generation": 3,
            "build_events": [self.workspace()] if ready else [],
            "recent_logs": [self.ready_log()] if ready else [],
        }

    def workspace(self):
        return {"type": "workspace", "data": {"build_dir": str(self.build_dir)}}

    def ready_log(self):
        return {
            "source": "daemon-metadata",
            "severity": "info",
            "message": "Initial workspace and recipe inventory ready",
        }

    def event(self, sequence, kind, data):
        return {
            "type": "event",
            "sequence": sequence,
            "generation": sequence,
            "event": {"type": kind, "data": data},
        }

    def client(self, messages=()):
        class Client:
            daemon_instance_id = self.instance

            def __init__(inner):
                inner.messages = iter(messages)
                inner.sent = []

            def receive(inner, timeout):
                item = next(inner.messages)
                if isinstance(item, BaseException):
                    raise item
                return item

            def send(inner, message):
                inner.sent.append(message)

        return Client()

    def wait(self, client, snapshot=None, **kwargs):
        return HARNESS.wait_for_metadata_ready(
            client,
            self.snapshot() if snapshot is None else snapshot,
            self.build_dir,
            **kwargs,
        )

    def test_delayed_workspace_and_ready_log_are_read_only_and_return_latest_generation(
        self,
    ):
        client = self.client(
            [
                None,
                {"type": "ping", "nonce": 7},
                self.event(4, "build", self.workspace()),
                self.event(5, "log", self.ready_log()),
            ]
        )
        self.assertEqual(self.wait(client), 5)
        self.assertEqual(client.sent, [{"type": "pong", "nonce": 7}])
        self.assertEqual(self.wait(self.client(), self.snapshot(ready=True)), 3)

    def test_replacement_snapshot_checks_the_full_instance_and_build_directory(self):
        snapshot = self.snapshot(ready=True)
        snapshot.update(type="snapshot", sequence=8, generation=8)
        self.assertEqual(self.wait(self.client([snapshot])), 8)
        for mutate, diagnostic in (
            (lambda s: s["daemon_instance_id"].__setitem__(15, 99), "instance"),
            (lambda s: s["daemon_instance_id"].__setitem__(1, True), "instance"),
            (
                lambda s: s["build_events"][0]["data"].update(build_dir="/other"),
                "build directory",
            ),
            (lambda s: s.update(generation=True), "generation"),
        ):
            with self.subTest(diagnostic=diagnostic):
                bad = self.snapshot(ready=True)
                mutate(bad)
                with self.assertRaisesRegex(RuntimeError, diagnostic):
                    self.wait(self.client(), bad)

    def test_failures_eof_timeout_and_message_flood_do_not_start_builds(self):
        failed = {
            "source": "daemon-metadata",
            "severity": "error",
            "message": "Initial metadata scan failed: fixture",
        }
        cases = [
            ([self.event(4, "log", failed)], "metadata"),
            ([{"type": "resync_required"}], "interrupted"),
            ([{"type": "shutting_down"}], "interrupted"),
            ([EOFError("closed")], "disconnected"),
            ([self.event(3, "build", self.workspace())], "sequence"),
        ]
        for messages, diagnostic in cases:
            with self.subTest(diagnostic=diagnostic):
                client = self.client(messages)
                with self.assertRaisesRegex(RuntimeError, diagnostic):
                    self.wait(client)
                self.assertEqual(client.sent, [])
        with patch.object(HARNESS.time, "monotonic", side_effect=[0, 21]):
            with self.assertRaisesRegex(RuntimeError, "timed out"):
                self.wait(self.client())
        noise = self.event(4, "telemetry", {})
        with self.assertRaisesRegex(RuntimeError, "message bound"):
            self.wait(self.client([noise]), max_messages=1)

    def test_ready_log_alone_and_unrelated_logs_are_not_workspace_authority(self):
        client = self.client(
            [
                self.event(4, "log", self.ready_log()),
                self.event(5, "build", self.workspace()),
            ]
        )
        self.assertEqual(self.wait(client), 5)
        spoof = self.ready_log() | {"source": "unrelated"}
        with self.assertRaisesRegex(RuntimeError, "message bound"):
            self.wait(self.client([self.event(4, "log", spoof)]), max_messages=1)

    def test_build_rejection_is_immediate_and_unrelated_requests_cannot_acknowledge(
        self,
    ):
        rejected = {
            "type": "command_result",
            "request_id": 1,
            "outcome": {
                "type": "rejected",
                "code": "conflict",
                "message": "Initial recipe inventory is still loading",
            },
        }
        with self.assertRaisesRegex(RuntimeError, "build request rejected.*conflict"):
            HARNESS.observe_build_ack(rejected, 1)
        accepted = {
            "type": "command_result",
            "request_id": 1,
            "outcome": {"type": "accepted"},
        }
        self.assertTrue(HARNESS.observe_build_ack(accepted, 1))
        self.assertFalse(HARNESS.observe_build_ack(accepted | {"request_id": 2}, 1))
        with self.assertRaisesRegex(RuntimeError, "protocol interrupted"):
            HARNESS.observe_build_ack({"type": "error", "message": "broken"}, 1)

    def test_attach_rejects_a_snapshot_from_another_full_instance(self):
        client = object.__new__(HARNESS.ProtocolClient)
        client.client_id = [9] * 16
        client.send = Mock()
        wrong = self.snapshot()
        wrong["daemon_instance_id"][-1] = 99
        client.receive = Mock(
            side_effect=[
                {"type": "hello", "daemon_instance_id": self.instance},
                {"type": "attached", "snapshot": wrong},
            ]
        )
        with self.assertRaisesRegex(RuntimeError, "instance differs"):
            client.attach()
        self.assertEqual(
            [call.args[0]["type"] for call in client.send.call_args_list],
            ["hello", "attach"],
        )


class EventFloodHarnessTests(unittest.TestCase):
    def test_progress_is_coalescible_but_terminal_and_failure_are_critical(
        self,
    ) -> None:
        self.assertNotIn("critical_task_progress", HARNESS.CRITICAL_NAMES)
        self.assertIn("critical_task_progress", HARNESS.COALESCIBLE_NAMES)
        for transition in ("critical_task_queued", "critical_task_started"):
            self.assertNotIn(transition, HARNESS.CRITICAL_NAMES)
            self.assertNotIn(transition, HARNESS.COALESCIBLE_NAMES)
            self.assertIn(transition, HARNESS.IMPORTANT_TRANSITION_NAMES)
        self.assertIn("critical_task_failed", HARNESS.CRITICAL_NAMES)
        self.assertIn("build_terminal", HARNESS.CRITICAL_NAMES)

    def environment(
        self,
        directory: str,
        *,
        duration: str = "0.25",
        profile: str = "balanced",
        terminal: str = "success",
    ) -> tuple[dict[str, str], Path]:
        report = Path(directory) / "report.json"
        environment = os.environ.copy()
        environment.update(
            {
                "YOCTUI_TEST_BUILD_DIR": directory,
                "YOCTUI_PERF_EVENT_RATE": "2000",
                "YOCTUI_PERF_EVENT_DURATION": duration,
                "YOCTUI_PERF_EVENT_PROFILE": profile,
                "YOCTUI_PERF_EVENT_TERMINAL": terminal,
                "YOCTUI_PERF_EVENT_REPORT": str(report),
            }
        )
        return environment, report

    def test_rejects_invalid_configuration(self) -> None:
        environment = os.environ.copy()
        environment["YOCTUI_PERF_EVENT_RATE"] = "0"
        result = subprocess.run(
            [str(FIXTURE)],
            cwd=ROOT,
            env=environment,
            text=True,
            input="",
            capture_output=True,
            timeout=3,
            check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("event rate must be positive", result.stderr)

    def test_balanced_stream_is_ordered_mixed_and_bounded(self) -> None:
        with tempfile.TemporaryDirectory(prefix="yoctui-event-mix-") as directory:
            environment, report_path = self.environment(directory)
            session = BridgeSession(environment)
            try:
                session.command("hello")
                self.assertEqual(session.event()["type"], "hello_ack")
                session.command("start_build")
                types: list[str] = []
                deadline = time.monotonic() + 4
                while time.monotonic() < deadline:
                    event = session.event()
                    types.append(str(event["type"]))
                    if event["type"] == "build_completed":
                        break
                else:
                    self.fail("fixture omitted its terminal event")
            finally:
                session.stop()
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(
                report["schema"], "yoctui.performance.event-flood-generator.v1"
            )
            self.assertGreaterEqual(report["measurement"]["ordinary_events"], 400)
            for required in (
                "task_queued",
                "task_started",
                "task_progress",
                "task_completed",
                "log",
                "warning",
                "error",
                "build_completed",
            ):
                self.assertIn(required, types)
            critical = [entry["name"] for entry in report["critical_sent"]]
            self.assertEqual(len(critical), len(set(critical)))
            self.assertEqual(critical[-1], "build_terminal")
            sequences = [entry["bridge_sequence"] for entry in report["critical_sent"]]
            self.assertEqual(sequences, sorted(sequences))

    def test_failure_cancellation_and_disconnect_are_explicit_outcomes(self) -> None:
        with tempfile.TemporaryDirectory(prefix="yoctui-event-cancel-") as directory:
            environment, report_path = self.environment(directory, duration="10")
            session = BridgeSession(environment)
            session.command("start_build")
            self.assertEqual(session.event()["type"], "build_started")
            session.command("cancel_build")
            terminal = None
            for _ in range(20):
                event = session.event()
                if event["type"] == "build_completed":
                    terminal = event
                    break
            session.stop()
            self.assertIsNotNone(terminal)
            self.assertEqual(terminal["exit_code"], 130)
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(report["outcome"], "cancelled")
            self.assertEqual(
                [entry["name"] for entry in report["critical_sent"]],
                ["cancellation", "cancellation_terminal"],
            )

        with tempfile.TemporaryDirectory(prefix="yoctui-event-failure-") as directory:
            environment, report_path = self.environment(
                directory, duration="0.1", terminal="failure"
            )
            session = BridgeSession(environment)
            session.command("start_build")
            terminal = None
            while terminal is None:
                event = session.event()
                if event["type"] == "build_completed":
                    terminal = event
            session.stop()
            self.assertFalse(terminal["success"])
            self.assertEqual(terminal["exit_code"], 1)
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(report["outcome"], "failed")

        with tempfile.TemporaryDirectory(
            prefix="yoctui-event-disconnect-"
        ) as directory:
            environment, report_path = self.environment(
                directory, duration="0.1", terminal="disconnect"
            )
            session = BridgeSession(environment)
            session.command("start_build")
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline and session.process.poll() is None:
                try:
                    session.event(0.1)
                except (AssertionError, EOFError):
                    pass
            self.assertEqual(session.process.wait(timeout=3), 0)
            report = json.loads(report_path.read_text(encoding="utf-8"))
            self.assertEqual(report["outcome"], "backend_disconnected")
            session.stop()


if __name__ == "__main__":
    unittest.main()
