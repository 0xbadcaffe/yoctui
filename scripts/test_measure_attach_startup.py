import importlib.util
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
spec = importlib.util.spec_from_file_location("measure_attach", Path(__file__).with_name("measure-attach-startup.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class AttachHealthTests(unittest.TestCase):
    def test_daemon_health_is_independent_of_idle_bitbake(self):
        self.assertTrue(module.daemon_connected("D:✓ Connected/Local • BB:– Disconnected"))
        self.assertTrue(module.daemon_connected("Daemon: ✓ Connected"))

    def test_disconnected_daemon_cannot_pass_on_connected_bitbake(self):
        self.assertFalse(module.daemon_connected("D:× Disconnected • BB:✓ Connected"))
        self.assertFalse(module.daemon_connected("Navigator • Loading…"))


if __name__ == "__main__":
    unittest.main()
