import importlib.util
from pathlib import Path
import unittest


def load_probe():
    path = Path(__file__).with_name("scheduler-latency-probe.py")
    spec = importlib.util.spec_from_file_location("scheduler_latency_probe", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


probe = load_probe()


class SchedulerLatencyProbeTests(unittest.TestCase):
    def test_percentile_uses_deterministic_nearest_rank(self):
        values = [5.0, 1.0, 4.0, 2.0, 3.0]
        self.assertEqual(probe.percentile(values, 0.50), 3.0)
        self.assertEqual(probe.percentile(values, 0.95), 5.0)

    def test_short_measurement_is_monotonic_and_complete(self):
        record = probe.measure(0.05, 0.01)
        self.assertEqual(record["schema"], probe.SCHEMA)
        self.assertEqual(record["clock"], "CLOCK_MONOTONIC")
        self.assertEqual(record["measurement"]["samples"], 5)
        latency = record["measurement"]["wake_latency_ms"]
        self.assertLessEqual(latency["p50"], latency["p95"])
        self.assertLessEqual(latency["p95"], latency["maximum"])


if __name__ == "__main__":
    unittest.main()
