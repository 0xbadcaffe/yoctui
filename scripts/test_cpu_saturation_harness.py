#!/usr/bin/env python3
"""Regression tests for the deterministic CPU saturation fixture."""

from __future__ import annotations

import json
import os
import signal
import subprocess
import tempfile
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
HARNESS = ROOT / "scripts/cpu-saturation-harness.py"


class CpuSaturationHarnessTests(unittest.TestCase):
    def run_harness(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(HARNESS), *arguments],
            cwd=ROOT,
            text=True,
            capture_output=True,
            timeout=15,
            check=False,
        )

    def test_rejects_invalid_worker_count(self) -> None:
        result = self.run_harness("--workers", "0")
        self.assertEqual(result.returncode, 2)
        self.assertIn("workers must be within", result.stderr)

    def test_oversubscribed_workers_cycle_over_selected_cpus(self) -> None:
        with tempfile.TemporaryDirectory(prefix="yoctui-saturation-over-") as directory:
            output = Path(directory) / "result.json"
            result = self.run_harness(
                "--workers",
                "2",
                "--cpu-list",
                str(next(iter(os.sched_getaffinity(0)))),
                "--warmup-seconds",
                "0",
                "--duration-seconds",
                "0.25",
                "--minimum-worker-cpu-percent",
                "10",
                "--output",
                str(output),
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            record = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(len(record["configuration"]["selected_cpus"]), 1)
            self.assertEqual(
                record["configuration"]["worker_cpu_assignments"],
                record["configuration"]["selected_cpus"] * 2,
            )
            self.assertEqual(len(record["workers"]), 2)

    def test_reports_ready_saturation_bounded_exit_and_cleanup(self) -> None:
        with tempfile.TemporaryDirectory(prefix="yoctui-saturation-test-") as directory:
            output = Path(directory) / "result.json"
            events = Path(directory) / "events.jsonl"
            result = self.run_harness(
                "--workers",
                "1",
                "--warmup-seconds",
                "0.1",
                "--duration-seconds",
                "0.5",
                "--minimum-worker-cpu-percent",
                "40",
                "--output",
                str(output),
                "--event-log",
                str(events),
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            record = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(record["schema"], "yoctui.performance.cpu-saturation.v1")
            self.assertEqual(record["status"], "completed")
            self.assertEqual(len(record["readiness"]), 1)
            self.assertEqual(len(record["workers"]), 1)
            self.assertGreater(record["workers"][0]["iterations"], 0)
            self.assertGreaterEqual(
                record["achieved"]["minimum_worker_cpu_percent"], 40
            )
            self.assertTrue(record["cleanup"]["children_reaped"])
            self.assertLess(record["timing"]["total_elapsed_seconds"], 4)
            kinds = [
                json.loads(line)["event"] for line in events.read_text().splitlines()
            ]
            self.assertIn("ready", kinds)
            self.assertEqual(kinds[-1], "completed")
            for pid in record["cleanup"]["worker_pids"]:
                self.assertFalse(Path(f"/proc/{pid}").exists())

    def test_sigterm_stops_and_reaps_ready_workers(self) -> None:
        with tempfile.TemporaryDirectory(prefix="yoctui-saturation-stop-") as directory:
            events = Path(directory) / "events.jsonl"
            process = subprocess.Popen(
                [
                    str(HARNESS),
                    "--workers",
                    "1",
                    "--warmup-seconds",
                    "0.1",
                    "--duration-seconds",
                    "10",
                    "--event-log",
                    str(events),
                ],
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                if events.exists() and '"event":"ready"' in events.read_text(
                    encoding="utf-8"
                ):
                    break
                time.sleep(0.02)
            else:
                process.kill()
                self.fail("fixture did not become ready")
            process.send_signal(signal.SIGTERM)
            stdout, stderr = process.communicate(timeout=5)
            self.assertEqual(process.returncode, 130, stderr)
            record = json.loads(stdout)
            self.assertEqual(record["status"], "interrupted")
            self.assertTrue(record["cleanup"]["children_reaped"])
            for pid in record["cleanup"]["worker_pids"]:
                self.assertFalse(Path(f"/proc/{pid}").exists())


if __name__ == "__main__":
    unittest.main()
