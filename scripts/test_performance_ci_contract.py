from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


class PerformanceCiContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        cls.fast = (ROOT / "scripts/verify-performance-ci-fast.sh").read_text(encoding="utf-8")

    def test_fast_job_covers_required_deterministic_paths(self) -> None:
        self.assertIn("performance-fast:", self.workflow)
        self.assertIn("./scripts/verify-performance-ci-fast.sh", self.workflow)
        for required in (
            "test-idle-event-loops.py",
            "render_scheduler",
            "task_batches_coalesce_progress_and_preserve_terminal_failures",
            "high_volume_logs_remain_within_retention_limits",
            "--harness",
            "--backpressure",
        ):
            self.assertIn(required, self.fast)

    def test_long_and_live_work_are_not_pull_request_gates(self) -> None:
        self.assertIn("performance-scheduled:", self.workflow)
        self.assertIn("performance-live-poky:", self.workflow)
        scheduled = self.workflow.split("performance-scheduled:", 1)[1]
        live = self.workflow.split("performance-live-poky:", 1)[1]
        condition = "github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'"
        self.assertIn(condition, scheduled)
        self.assertIn(condition, live)
        self.assertIn("./scripts/verify-low-overhead.sh", scheduled)
        self.assertIn("--sample-seconds 1800", scheduled)
        self.assertIn("--real-poky-evidence", scheduled)
        self.assertIn("capture-real-poky-performance.py", live)
        self.assertIn("YOCTUI_LIVE_PERFORMANCE", live)

    def test_every_performance_job_uploads_failure_evidence(self) -> None:
        performance = self.workflow.split("performance-fast:", 1)[1]
        self.assertGreaterEqual(performance.count("actions/upload-artifact@v4"), 3)
        self.assertGreaterEqual(performance.count("if: always()"), 3)
        self.assertIn("artifacts/performance/ci/", performance)


if __name__ == "__main__":
    unittest.main()
