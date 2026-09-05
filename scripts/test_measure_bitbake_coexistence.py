import importlib.util
from pathlib import Path
import unittest


def load_measurement():
    path = Path(__file__).with_name("measure-bitbake-coexistence.py")
    spec = importlib.util.spec_from_file_location("measure_bitbake_coexistence", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


measurement = load_measurement()


class BitBakeCoexistenceMeasurementTests(unittest.TestCase):
    def test_summary_uses_median_and_preserves_worst_tail(self):
        trials = [
            {"measurement": {"wake_latency_ms": {"p95": p95, "maximum": maximum}}}
            for p95, maximum in ((2.0, 7.0), (1.0, 9.0), (3.0, 8.0))
        ]
        self.assertEqual(
            measurement.scenario_summary(trials),
            {
                "median_p95_wake_latency_ms": 2.0,
                "worst_p95_wake_latency_ms": 3.0,
                "worst_maximum_wake_latency_ms": 9.0,
            },
        )

    def test_load_summary_retains_worker_to_cpu_oversubscription(self):
        record = {
            "status": "completed",
            "configuration": {
                "requested_workers": 4,
                "selected_cpus": [2, 3],
                "worker_cpu_assignments": [2, 3, 2, 3],
            },
            "achieved": {
                "minimum_worker_cpu_percent": 40.0,
                "mean_worker_cpu_percent": 49.0,
                "host_cpu_utilization_percent": 100.0,
                "load_average_before": [2.0, 1.0, 1.0],
                "load_average_after": [4.0, 2.0, 1.0],
            },
            "cleanup": {"children_reaped": True},
        }
        summary = measurement.load_summary(record)
        self.assertEqual(summary["requested_workers"], 4)
        self.assertEqual(summary["worker_cpu_assignments"], [2, 3, 2, 3])
        self.assertTrue(summary["children_reaped"])


if __name__ == "__main__":
    unittest.main()
