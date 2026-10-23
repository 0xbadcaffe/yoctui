"""Keep real BitBake logging arguments in the displayed diagnostic."""
import importlib.util
import logging
import unittest

from .support import BRIDGE


class LogRecordMessageTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("bridge_log_messages", BRIDGE)
        cls.bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.bridge)

    def test_build_summary_formats_real_logrecord_arguments(self):
        record = logging.LogRecord(
            "BitBake", logging.INFO, "/bitbake/runqueue.py", 1,
            "Tasks Summary: Attempted %d tasks; %s.", (4404, "all succeeded"), None,
        )
        record.message = "stale cached message"
        normalized = self.bridge.normalize_event(record)
        self.assertEqual(normalized["type"], "log")
        self.assertEqual(normalized["message"],
                         "Tasks Summary: Attempted 4404 tasks; all succeeded.")

    def test_bad_format_arguments_preserve_raw_diagnostic(self):
        record = logging.LogRecord(
            "BitBake", logging.WARNING, "/bitbake/runqueue.py", 1,
            "bad count %d", ("not an integer",), None,
        )
        normalized = self.bridge.normalize_event(record)
        self.assertEqual(normalized["type"], "log")
        self.assertEqual(normalized["message"], "bad count %d")
        self.assertEqual(normalized["level"], "warning")


if __name__ == "__main__":
    unittest.main()
