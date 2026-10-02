#!/usr/bin/env python3
"""Focused framing and read-only authority guards for ZCU102 evidence."""
import importlib.util
import json
from pathlib import Path
import struct
import unittest

spec = importlib.util.spec_from_file_location(
    "inspector", Path(__file__).with_name("inspect-zcu102-build.py"))
inspector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inspector)


class Connection:
    def __init__(self, messages=(), raw=None):
        self.data = bytearray(raw if raw is not None else b"".join(
            struct.pack(">I", len(payload)) + payload
            for payload in (json.dumps(message).encode() for message in messages)))
        self.sent = []

    def recv(self, count):
        result = bytes(self.data[:min(count, 3)])
        del self.data[:len(result)]
        return result

    def sendall(self, frame):
        self.sent.append(json.loads(frame[4:]))


class InspectorTests(unittest.TestCase):
    def connection(self, build="/validation/build", instance=None):
        return Connection([
            {"type": "hello", "selected_version": inspector.VERSION,
             "daemon_instance_id": [1] * 16},
            {"type": "attached", "snapshot": {
                "daemon_instance_id": instance or [1] * 16, "sequence": 40,
                "workspace": {"canonical_build": build},
                "build_progress": {"completed": 10, "total": 100},
                "build_events": [{"type": "task_completed", "success": False}],
            }},
        ])

    def test_partial_reads_keep_actual_progress_and_failures_without_commands(self):
        connection = self.connection()
        result = inspector.inspect(connection, Path("/validation/build"))
        self.assertEqual(result["build_progress"], {"completed": 10, "total": 100})
        self.assertEqual(len(result["recent_task_failures"]), 1)
        self.assertEqual([v["type"] for v in connection.sent], ["hello", "attach", "detach"])

    def test_other_build_rejected(self):
        with self.assertRaisesRegex(ValueError, "workspace"):
            inspector.inspect(self.connection("/other/build"), Path("/validation/build"))

    def test_instance_change_rejected(self):
        with self.assertRaisesRegex(ValueError, "instance"):
            inspector.inspect(self.connection(instance=[2] * 16), Path("/validation/build"))

    def test_oversize_and_empty_frames_rejected(self):
        for size in (0, inspector.MAX_FRAME + 1):
            with self.subTest(size=size), self.assertRaises(ValueError):
                inspector.receive(Connection(raw=struct.pack(">I", size)))

    def test_partial_frame_eof_rejected(self):
        with self.assertRaises(EOFError):
            inspector.receive(Connection(raw=struct.pack(">I", 20) + b"{}"))

    def test_handshake_rejected(self):
        with self.assertRaisesRegex(ValueError, "handshake"):
            inspector.inspect(Connection([{"type": "error"}]), Path("/validation/build"))


if __name__ == "__main__":
    unittest.main()
