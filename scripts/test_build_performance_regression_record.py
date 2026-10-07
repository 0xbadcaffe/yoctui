from __future__ import annotations

import importlib.util
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/build-performance-regression-record.py"
SPEC = importlib.util.spec_from_file_location("performance_regression", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PerformanceRegressionRecordTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory(prefix="yoctui-performance-fixture-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        process = {
            "cpu_trimmed_mean_percent_one_logical_cpu": 0.1,
            "voluntary_context_switches_mean_per_second": 1,
        }
        latency = {"p95": 20}
        memory = {
            "rss_growth_bytes": 1024,
            "final_window_slope_bytes_per_minute": 512,
            "rss_max_bytes": 8192,
        }
        # Controlled inputs exercise the report schema and gates without
        # treating measurements from an older checkout as current evidence.
        records = {
            "idle_suite": {"method": {"fixture": True}},
            "idle_daemon": {"summary": {"processes": {"daemon": process}}},
            "idle_attached": {
                "summary": {
                    "processes": {"daemon": process, "client": process},
                    "combined_cpu_trimmed_mean_percent_one_logical_cpu": 0.2,
                }
            },
            "input_latency": {
                "configuration": {"fixture": True},
                "summary": {
                    name: latency
                    for name in (
                        "keyboard_to_model_ms",
                        "keyboard_to_visible_frame_ms",
                        "mouse_to_visible_selection_ms",
                    )
                },
            },
            "ipc_latency": {
                "configuration": {"fixture": True},
                "summary": {
                    name: latency
                    for name in (
                        "daemon_event_to_client_ms",
                        "client_command_to_daemon_ms",
                        "cancellation_request_to_ack_ms",
                    )
                },
            },
            "ipc_continuity": {
                "observations": {
                    "maximum_queue_depth": 64,
                    "forced_resynchronizations": 0,
                    "reconnect_succeeded": True,
                }
            },
            "memory_endurance": {
                "configuration": {"fixture": True},
                "summary": {"daemon": memory, "client": memory},
                "retention": {
                    "critical_retention_passed": True,
                    "strict_event_order": True,
                    "connection_continuity": True,
                },
            },
            "real_poky": {
                "measurement": {"fixture": True},
                "summary": {
                    "daemon": process,
                    "client": process,
                    "combined_cpu_trimmed_mean_percent_one_logical_cpu": 0.2,
                    "host_cpu_trimmed_mean_percent_total_capacity": 99,
                    "bitbake_cpu_trimmed_mean_percent_one_logical_cpu": 100,
                },
                "responsiveness": {
                    "key_to_visible_frame_p95_ms": 20,
                    "ipc_ms": {"cancellation_command_to_ack_ms": 20},
                },
                "rendering": {"frames_per_second": 4, "frames": 40, "coalesced": 10},
                "pressure": {"maximum_queue_depth": 64, "forced_resynchronizations": 0},
                "continuity": {
                    "backend_disconnects": 0,
                    "client_reconnected": True,
                    "cancellation": {"requested": True, "acknowledged": True},
                },
            },
        }
        for name, relative in MODULE.SOURCES.items():
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(records[name]), encoding="utf-8")
        root_patch = patch.object(MODULE, "ROOT", self.root)
        root_patch.start()
        self.addCleanup(root_patch.stop)

    def test_record_is_complete_and_all_hard_gates_pass(self) -> None:
        record = MODULE.build_record()
        self.assertEqual(record["schema"], "yoctui.performance.regression-record.v1")
        self.assertEqual(
            set(record["metrics"]),
            {"cpu", "latency", "wakeups", "render", "pressure", "memory"},
        )
        self.assertGreaterEqual(record["result"]["hard_metrics"], 20)
        self.assertEqual(
            record["result"]["hard_metrics"], record["result"]["hard_metrics_passed"]
        )
        self.assertTrue(record["result"]["passed"])

    def test_cli_output_is_deterministic_and_bound_to_source_hashes(self) -> None:
        first = MODULE.build_record()
        second = MODULE.build_record()
        self.assertEqual(first, second)
        for source in first["source_artifacts"].values():
            self.assertEqual(len(source["sha256"]), 64)
            path = self.root / source["path"]
            self.assertEqual(
                source["sha256"], hashlib.sha256(path.read_bytes()).hexdigest()
            )
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "record.json"
            self.assertEqual(MODULE.main.__annotations__["return"], "int")
            destination.write_text(json.dumps(first, sort_keys=True), encoding="utf-8")
            self.assertEqual(json.loads(destination.read_text(encoding="utf-8")), first)

    def test_exceeded_limit_fails_the_aggregate_result(self) -> None:
        path = self.root / MODULE.SOURCES["input_latency"]
        record = json.loads(path.read_text())
        record["summary"]["keyboard_to_visible_frame_ms"]["p95"] = 101
        path.write_text(json.dumps(record))
        result = MODULE.build_record()
        self.assertFalse(result["metrics"]["latency"]["key_to_frame_p95"]["passed"])
        self.assertFalse(result["result"]["passed"])

    def test_failed_correctness_check_fails_the_aggregate_result(self) -> None:
        path = self.root / MODULE.SOURCES["memory_endurance"]
        record = json.loads(path.read_text())
        record["retention"]["critical_retention_passed"] = False
        path.write_text(json.dumps(record))
        self.assertFalse(MODULE.build_record()["result"]["passed"])


if __name__ == "__main__":
    unittest.main()
