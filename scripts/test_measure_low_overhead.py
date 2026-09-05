import importlib.util
from pathlib import Path
import unittest


def load_measurement():
    path = Path(__file__).with_name("measure-low-overhead.py")
    spec = importlib.util.spec_from_file_location("measure_low_overhead", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


measurement = load_measurement()


def record(daemon: float, client: float | None = None):
    processes = {
        "daemon": {"cpu_trimmed_mean_percent_one_logical_cpu": daemon}
    }
    combined = daemon
    if client is not None:
        processes["client"] = {
            "cpu_trimmed_mean_percent_one_logical_cpu": client
        }
        combined += client
    return {
        "summary": {
            "combined_cpu_trimmed_mean_percent_one_logical_cpu": combined,
            "processes": processes,
        }
    }


class LowOverheadMeasurementTests(unittest.TestCase):
    def test_thresholds_are_percent_of_one_cpu_and_accept_boundary(self):
        values = measurement.validate_thresholds(
            record(0.20), record(0.20, 0.50)
        )
        self.assertEqual(values["combined_cpu_percent_one_logical_cpu"], 0.70)

    def test_idle_daemon_threshold_is_independent(self):
        with self.assertRaisesRegex(RuntimeError, "idle_daemon"):
            measurement.validate_thresholds(record(0.21), record(0.10, 0.20))

    def test_combined_threshold_is_not_multiplied_by_cpu_count(self):
        with self.assertRaisesRegex(RuntimeError, "combined_cpu"):
            measurement.validate_thresholds(record(0.10), record(0.60, 0.50))


if __name__ == "__main__":
    unittest.main()
