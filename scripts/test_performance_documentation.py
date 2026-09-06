from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


class PerformanceDocumentationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.performance = (ROOT / "docs/performance.md").read_text(encoding="utf-8")
        cls.architecture = (ROOT / "docs/architecture.md").read_text(encoding="utf-8")
        cls.ui = (ROOT / "docs/ui-spec.md").read_text(encoding="utf-8")
        cls.performance_flat = " ".join(cls.performance.split())
        cls.architecture_flat = " ".join(cls.architecture.split())

    def test_cpu_accounting_and_scenarios_are_explicit(self) -> None:
        for required in (
            "one percent of one logical CPU",
            "10% trimmed-mean CPU",
            "10-second warmup",
            "60 one-second samples",
            "Idle daemon",
            "Idle attached client",
            "Active build",
            "PTY attached but idle",
            "High-rate BitBake stream",
            "Two attached clients",
        ):
            self.assertIn(required, self.performance_flat)

    def test_runtime_policy_matches_implementation(self) -> None:
        for document in (self.performance, self.architecture, self.ui):
            self.assertIn("4 Hz", document)
            self.assertIn("1 Hz", document)
        self.assertNotIn("100 ms minimum normal frame", self.performance)
        self.assertNotIn("200\nms (5 Hz)", self.performance)
        self.assertIn("250 ms normal frame interval", self.performance)
        self.assertIn("35 ms supervisor-service bound", self.performance)

    def test_backpressure_and_tuning_preserve_correctness_and_authority(self) -> None:
        for required in (
            "Slow clients cannot",
            "never dropped",
            "No supported path requires root",
            "never changes them",
            "real-Poky",
            "fixture evidence",
        ):
            self.assertIn(required, self.performance)
        self.assertIn("Progress", self.architecture_flat)
        self.assertIn("correctness sentinels", self.architecture_flat)

    def test_all_operator_gates_and_profile_paths_are_reproducible(self) -> None:
        for command in (
            "./scripts/verify-low-overhead.sh",
            "./scripts/verify-saturation-responsiveness.sh",
            "./scripts/verify-ipc-continuity.sh",
            "./scripts/verify-bounded-memory.sh",
            "./scripts/capture-runtime-profile.sh",
            "./scripts/capture-real-poky-performance.py",
            "./scripts/verify-performance.sh --regressions",
            "./scripts/verify-performance.sh --ci",
        ):
            self.assertIn(command, self.performance)


if __name__ == "__main__":
    unittest.main()
