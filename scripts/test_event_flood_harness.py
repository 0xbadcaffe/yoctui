#!/usr/bin/env python3
"""Regression tests for the deterministic BitBake-like event flood fixture."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "scripts/fixtures/bitbake-event-flood-bridge.py"


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


class EventFloodHarnessTests(unittest.TestCase):
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
