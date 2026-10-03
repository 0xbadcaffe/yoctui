"""Native disk and inode guard event regression coverage."""

from .support import *  # noqa: F403


class BridgeDiskGuardEventTests(unittest.TestCase):  # noqa: F405
    def disk_guard_bridge(self):
        spec = importlib.util.spec_from_file_location("yoctui_bridge_disk_guard", BRIDGE)
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        return bridge

    def test_disk_guard_normalizes_native_disk_and_inode_without_task_failure(self):
        bridge = self.disk_guard_bridge()
        self.assertIn("bb.event.DiskFull", bridge.TinfoilConnection.EVENT_MASK)
        for resource, unit in (("disk", "bytes"), ("inode", "inodes")):
            event = type("DiskFull", (), {
                "_type": resource, "_free": 42, "_mountpoint": "/build/tmp",
            })()
            normalized = bridge.normalize_event(event)
            self.assertEqual(normalized["type"], "log")
            self.assertEqual(normalized["level"], "error")
            self.assertIn(f"remaining=42 {unit}", normalized["message"])
            self.assertIn("runqueue is incomplete", normalized["message"])
            self.assertIsNone(normalized["recipe"])
            self.assertIsNone(normalized["task"])

    def test_disk_guard_soft_stop_overrides_zero_and_explicit_success(self):
        bridge = self.disk_guard_bridge()
        for explicit in (None, True):
            with self.subTest(explicit=explicit):
                adapter = bridge.BitBakeAdapter("2.8.1")
                completion = {"type": "build_completed", "_failures": 0,
                              "_interrupted": 0, "returncode": 0}
                if explicit is not None:
                    completion["success"] = explicit
                adapter.connection = SimpleNamespace(
                    start_build=lambda *_a: None,
                    drain_events=lambda: [{"type": "disk_full", "_type": "disk"}, completion],
                )
                adapter.start_build(["petalinux-image-minimal"], None)
                events = adapter.native_events()
                self.assertEqual([e["type"] for e in events], ["log", "build_completed"])
                self.assertEqual(events[-1], {"type": "build_completed", "success": False, "exit_code": 1})
                self.assertFalse(adapter.build_active)

    def test_disk_guard_does_not_change_ordinary_failure_or_cancellation(self):
        bridge = self.disk_guard_bridge()
        for completion in (
            {"type": "build_completed", "success": False, "exit_code": 7},
            {"type": "build_completed", "_failures": 0, "_interrupted": 1},
        ):
            with self.subTest(completion=completion):
                adapter = bridge.BitBakeAdapter("2.8.1")
                adapter.connection = SimpleNamespace(
                    start_build=lambda *_a: None,
                    drain_events=lambda: [completion],
                )
                adapter.start_build(["image"], None)
                self.assertEqual(adapter.native_events(), [bridge.normalize_event(completion)])
                self.assertFalse(adapter.disk_guard_stopped)

    def test_disk_guard_unknown_metadata_remains_explicit_and_bounded(self):
        bridge = self.disk_guard_bridge()
        for free in (None, True, -1, float("nan"), "invalid"):
            normalized = bridge.normalize_event({
                "type": "disk_full", "_type": None, "_free": free,
                "_mountpoint": "x" * 5000,
            })
            self.assertIn("remaining=unknown units", normalized["message"])
            self.assertLess(len(normalized["message"]), 4300)

    def test_disk_guard_survives_poll_boundary_and_resets_for_next_build(self):
        bridge = self.disk_guard_bridge()
        batches = iter([
            [{"type": "disk_full", "_type": "inode"}],
            [{"type": "build_completed", "success": True, "exit_code": 0}],
            [{"type": "build_completed", "success": True, "exit_code": 0}],
        ])
        adapter = bridge.BitBakeAdapter("2.8.1")
        adapter.connection = SimpleNamespace(
            start_build=lambda *_a: None, drain_events=lambda: next(batches, []),
        )
        adapter.start_build(["first"], None)
        self.assertEqual(adapter.native_events()[0]["type"], "log")
        self.assertTrue(adapter.build_active)
        self.assertFalse(adapter.native_events()[-1]["success"])
        adapter.start_build(["second"], None)
        self.assertFalse(adapter.disk_guard_stopped)
        self.assertTrue(adapter.native_events()[-1]["success"])

    def test_disk_guard_outside_active_build_does_not_poison_success(self):
        bridge = self.disk_guard_bridge()
        batches = iter([
            [{"type": "disk_full"}],
            [{"type": "build_completed", "success": True, "exit_code": 0}],
        ])
        adapter = bridge.BitBakeAdapter("2.8.1")
        adapter.connection = SimpleNamespace(
            start_build=lambda *_a: None, drain_events=lambda: next(batches, []),
        )
        self.assertEqual(adapter.native_events(), [])
        self.assertFalse(adapter.disk_guard_stopped)
        adapter.start_build(["new"], None)
        self.assertTrue(adapter.native_events()[-1]["success"])

    def test_disk_guard_fake_bridge_emits_failed_completion_after_native_zero(self):
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                '''__version__ = "2.8.1"
class DiskFull:
 def __init__(self): self._type="disk"; self._free=3_000_000_000; self._mountpoint="/build/tmp"
class BuildCompleted:
 def __init__(self): self._failures=0; self._interrupted=0
class Connection:
 def start_build(self, targets, task): pass
 def drain_events(self): return [DiskFull(), BuildCompleted()]
class Server:
 def connect(self): return Connection()
server = Server()
''', encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":["petalinux-image-minimal"],"task":null}}',
                environment={"PYTHONPATH": directory},
            )
        self.assertEqual(result.returncode, 0)  # Bridge transport remains healthy.
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual([m["type"] for m in messages], ["build_started", "log", "build_completed"])
        self.assertEqual(messages[-1], {"type": "build_completed", "success": False, "exit_code": 1})
        self.assertIn("guard stopped", messages[1]["message"])
