"""Bridge framing tests; compatible with both unittest and pytest collection."""
import json
import subprocess
import sys
import unittest
from pathlib import Path


BRIDGE = Path(__file__).parents[1] / "ratabake_bridge.py"


def run_bridge(*lines: bytes) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [sys.executable, str(BRIDGE)],
        input=b"".join(line + b"\n" for line in lines),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


class BridgeProtocolTests(unittest.TestCase):
    def test_hello_and_shutdown_are_framed_as_json_lines(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"hello"}}',
            b'{"protocol_version":1,"sequence":2,"message":{"type":"shutdown"}}',
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, b"")
        messages = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual([m["message"]["type"] for m in messages], ["hello_ack", "bridge_shutdown"])
        self.assertEqual([m["sequence"] for m in messages], [1, 2])

    def test_malformed_input_is_reported_without_crashing(self) -> None:
        result = run_bridge(b"not json")
        self.assertEqual(result.returncode, 0)
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["type"], "command_failed")
        self.assertEqual(message["message"]["code"], "malformed_command")

    def test_unknown_command_is_typed_error(self) -> None:
        result = run_bridge(b'{"protocol_version":1,"sequence":1,"message":{"type":"future"}}')
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["code"], "unknown_command")

    def test_protocol_version_mismatch_is_rejected(self) -> None:
        result = run_bridge(b'{"protocol_version":999,"sequence":1,"message":{"type":"hello"}}')
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["code"], "version_mismatch")

    def test_parent_eof_exits_cleanly(self) -> None:
        result = run_bridge()
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")

