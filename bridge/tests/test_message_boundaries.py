"""The Python receiver shares Rust's payload budget, excluding the newline."""

from .support import MAX_LINE_BYTES, json, run_bridge, unittest


class BridgeMessageBoundaryTests(unittest.TestCase):
    def payload(self, size: int, character: str = "x") -> bytes:
        message = {
            "protocol_version": 1,
            "sequence": 1,
            "message": {"type": "boundary_test_unknown_command"},
            "padding": "",
        }
        overhead = len(json.dumps(message, ensure_ascii=False).encode("utf-8"))
        count, remainder = divmod(size - overhead, len(character.encode("utf-8")))
        message["padding"] = character * count + "x" * remainder
        encoded = json.dumps(message, ensure_ascii=False).encode("utf-8")
        self.assertEqual(len(encoded), size)
        return encoded

    def test_exact_payload_limit_excludes_the_newline(self) -> None:
        result = run_bridge(self.payload(MAX_LINE_BYTES))
        self.assertEqual(result.returncode, 0)
        self.assertEqual(
            json.loads(result.stdout)["message"]["code"], "unknown_command"
        )

    def test_exact_unicode_payload_limit_counts_utf8_bytes(self) -> None:
        result = run_bridge(self.payload(MAX_LINE_BYTES, "茶"))
        self.assertEqual(result.returncode, 0)
        self.assertEqual(
            json.loads(result.stdout)["message"]["code"], "unknown_command"
        )

    def test_legacy_payload_one_byte_under_limit_is_still_accepted(self) -> None:
        result = run_bridge(self.payload(MAX_LINE_BYTES - 1))
        self.assertEqual(
            json.loads(result.stdout)["message"]["code"], "unknown_command"
        )

    def test_payload_one_byte_over_limit_is_rejected(self) -> None:
        result = run_bridge(self.payload(MAX_LINE_BYTES + 1))
        self.assertEqual(result.returncode, 0)
        self.assertEqual(
            json.loads(result.stdout)["message"]["code"], "message_too_large"
        )

    def test_rejected_message_does_not_consume_the_following_command(self) -> None:
        shutdown = b'{"protocol_version":1,"sequence":2,"message":{"type":"shutdown"}}'
        result = run_bridge(self.payload(MAX_LINE_BYTES + 1), shutdown)
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(result.returncode, 0)
        self.assertEqual(messages[0]["code"], "message_too_large")
        self.assertEqual(messages[1]["type"], "bridge_shutdown")

    def test_invalid_utf8_at_the_payload_limit_is_a_decode_error_not_size_error(
        self,
    ) -> None:
        result = run_bridge(b"\xff" * MAX_LINE_BYTES)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(
            json.loads(result.stdout)["message"]["code"], "malformed_command"
        )
