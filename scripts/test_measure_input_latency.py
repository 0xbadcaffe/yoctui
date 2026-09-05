import importlib.util
from pathlib import Path
import unittest


def load_measurement():
    path = Path(__file__).with_name("measure-input-latency.py")
    spec = importlib.util.spec_from_file_location("measure_input_latency", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


measurement = load_measurement()


class InputLatencyMeasurementTests(unittest.TestCase):
    def test_nearest_rank_percentiles_preserve_tail(self):
        values = [float(value) for value in range(1, 101)]
        self.assertEqual(measurement.summary(values)["p50"], 50.0)
        self.assertEqual(measurement.summary(values)["p95"], 95.0)
        self.assertEqual(measurement.summary(values)["maximum"], 100.0)

    def test_marker_parser_preserves_kind_sequence_and_monotonic_fields(self):
        marker = measurement.MARKER.search(
            b"noise\x1b]777;yoctui-input-latency;mouse;7;100;110;125;3\x07"
        )
        self.assertIsNotNone(marker)
        self.assertEqual(marker.group(1), b"mouse")
        self.assertEqual(marker.group(2), b"7")
        self.assertEqual(marker.group(6), b"3")


if __name__ == "__main__":
    unittest.main()
