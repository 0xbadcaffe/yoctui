from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/build-performance-regression-record.py"
SPEC = importlib.util.spec_from_file_location("performance_regression", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PerformanceRegressionRecordTests(unittest.TestCase):
    def test_record_is_complete_and_all_hard_gates_pass(self) -> None:
        record = MODULE.build_record()
        self.assertEqual(record["schema"], "yoctui.performance.regression-record.v1")
        self.assertEqual(
            set(record["metrics"]), {"cpu", "latency", "wakeups", "render", "pressure", "memory"}
        )
        self.assertGreaterEqual(record["result"]["hard_metrics"], 20)
        self.assertEqual(record["result"]["hard_metrics"], record["result"]["hard_metrics_passed"])
        self.assertTrue(record["result"]["passed"])

    def test_cli_output_is_deterministic_and_bound_to_source_hashes(self) -> None:
        first = MODULE.build_record()
        second = MODULE.build_record()
        self.assertEqual(first, second)
        for source in first["source_artifacts"].values():
            self.assertEqual(len(source["sha256"]), 64)
            self.assertTrue((ROOT / source["path"]).is_file())
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "record.json"
            self.assertEqual(MODULE.main.__annotations__["return"], "int")
            destination.write_text(json.dumps(first, sort_keys=True), encoding="utf-8")
            self.assertEqual(json.loads(destination.read_text(encoding="utf-8")), first)


if __name__ == "__main__":
    unittest.main()
