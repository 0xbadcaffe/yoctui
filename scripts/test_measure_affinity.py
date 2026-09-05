import importlib.util
from pathlib import Path
import unittest


def load_measurement():
    path = Path(__file__).with_name("measure-affinity.py")
    spec = importlib.util.spec_from_file_location("measure_affinity", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


measurement = load_measurement()


class AffinityMeasurementTests(unittest.TestCase):
    def test_cpu_list_preserves_explicit_logical_cpu_identity(self):
        self.assertEqual(measurement.cpu_list([0, 2, 7]), "0,2,7")

    def test_summary_uses_median_trials_and_worst_tail(self):
        trials = [
            {
                "measurement": {
                    "wake_latency_ms": {"p95": p95, "maximum": maximum}
                }
            }
            for p95, maximum in ((3.0, 8.0), (1.0, 9.0), (2.0, 7.0))
        ]
        summary = measurement.percentile_summary(trials)
        self.assertEqual(summary["median_p95_wake_latency_ms"], 2.0)
        self.assertEqual(summary["worst_p95_wake_latency_ms"], 3.0)
        self.assertEqual(summary["worst_maximum_wake_latency_ms"], 9.0)


if __name__ == "__main__":
    unittest.main()
