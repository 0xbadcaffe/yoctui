"""Negative checks for the actual release workflow toolchain/lint contract."""
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/ci.yml"
CHECKER = compile(
    (ROOT / "scripts/check-ci.sh").read_text().split("python3 <<'PY'\n", 1)[1].split("\nPY", 1)[0],
    "check-ci.sh", "exec",
)


class ToolchainContractTests(unittest.TestCase):
    def run_contract(self, workflow):
        original = Path.read_text
        with patch.object(Path, "read_text", autospec=True) as reader:
            reader.side_effect = lambda path, **kwargs: workflow if path == WORKFLOW else original(path, **kwargs)
            with redirect_stdout(StringIO()):
                exec(CHECKER, {})

    def test_pinned_workflow_passes(self):
        self.run_contract(WORKFLOW.read_text())

    def test_one_floating_job_fails(self):
        workflow = WORKFLOW.read_text().replace("@1.97.0", "@stable", 1)
        with self.assertRaisesRegex(SystemExit, "every release job"):
            self.run_contract(workflow)

    def test_removing_strict_clippy_fails(self):
        workflow = WORKFLOW.read_text().replace(" -- -D warnings", "")
        with self.assertRaisesRegex(SystemExit, "strict workspace Clippy"):
            self.run_contract(workflow)

    def test_bridge_discovery_requires_package_top_level(self):
        workflow = WORKFLOW.read_text().replace("discover -s bridge/tests -t .", "discover bridge/tests")
        with self.assertRaisesRegex(SystemExit, "workflow lacks"):
            self.run_contract(workflow)


if __name__ == "__main__":
    unittest.main()
