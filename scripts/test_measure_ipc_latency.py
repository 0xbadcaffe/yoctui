import importlib.util
from pathlib import Path
import unittest
from unittest.mock import Mock


def load_measurement():
    path = Path(__file__).with_name("measure-ipc-latency.py")
    spec = importlib.util.spec_from_file_location("measure_ipc_latency", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


measurement = load_measurement()


class IpcLatencyMeasurementTests(unittest.TestCase):
    def test_live_handshake_uses_current_daemon_protocol(self):
        client = object.__new__(measurement.ProtocolClient)
        client.client_id = [71] * 16
        client.send = Mock()
        client.receive = Mock(side_effect=[
            ({"type": "hello"}, 0),
            ({"type": "attached", "snapshot": {"generation": 1}}, 0),
        ])
        self.assertEqual(client.attach(), {"generation": 1})
        hello = client.send.call_args_list[0].args[0]
        source = (Path(__file__).resolve().parents[1] /
                  "crates/yoctui-protocol/src/daemon.rs").read_text()
        for name in ("major", "minor"):
            expected = int(source.split(f"pub const PROTOCOL_{name.upper()}: u16 = ")[1].split(";")[0])
            self.assertEqual(hello["minimum_version"][name], expected)
            self.assertEqual(hello["maximum_version"][name], expected)

    def test_nearest_rank_percentiles_preserve_tail(self):
        values = [float(value) for value in range(1, 101)]
        result = measurement.summarize(values)
        self.assertEqual(result["p50"], 50.0)
        self.assertEqual(result["p95"], 95.0)
        self.assertEqual(result["maximum"], 100.0)

    def test_latency_marker_requires_exact_sequence_and_timestamp(self):
        self.assertEqual(
            measurement.parse_latency_marker("PERF_IPC_LATENCY:17:123456"),
            (17, 123456),
        )
        self.assertIsNone(measurement.parse_latency_marker("ordinary log"))
        self.assertIsNone(measurement.parse_latency_marker("PERF_IPC_LATENCY:x:1"))

    def test_event_sequence_extraction_uses_protocol_envelope(self):
        message = {
            "type": "event",
            "sequence": 81,
            "event": {
                "type": "log",
                "data": {"message": "PERF_IPC_LATENCY:3:900"},
            },
        }
        self.assertEqual(measurement.latency_event(message), (81, 3, 900))

    def test_command_outcome_identity_preserves_rejection_code(self):
        self.assertEqual(
            measurement.outcome_identity(
                {
                    "type": "command_result",
                    "request_id": 7,
                    "outcome": {"type": "rejected", "code": "not_found"},
                }
            ),
            ("rejected", "not_found"),
        )


if __name__ == "__main__":
    unittest.main()
