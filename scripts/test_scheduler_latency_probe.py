import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch


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

    def test_exact_period_counts_do_not_depend_on_clock_magnitude(self):
        for started in (0.0, 1.0, 123456.0):
            for duration, interval, samples in (
                (0.05, 0.01, 5),
                (0.3, 0.1, 3),
                (0.055, 0.01, 5),
            ):
                with (
                    self.subTest(started=started, duration=duration),
                    patch.object(probe.time, "monotonic", return_value=started),
                    patch.object(probe.time, "sleep") as sleep,
                ):
                    record = probe.measure(duration, interval)
                    self.assertEqual(record["measurement"]["samples"], samples)
                    self.assertEqual(sleep.call_count, samples)


if __name__ == "__main__":
    unittest.main()
