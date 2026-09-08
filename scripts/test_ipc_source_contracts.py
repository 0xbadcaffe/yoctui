"""Exercise the actual shell gate's Python source checks, including mutations."""

from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
SUPERVISOR = "crates/yoctui-cli/src/daemon_bitbake.rs"
TRANSPORT = "crates/yoctui-protocol/src/daemon_ipc.rs"
DAEMON = "crates/yoctui-cli/src/main.rs"


class IpcSourceContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        script = (ROOT / "scripts/verify-ipc-continuity.sh").read_text()
        function = script.split("verify_source_and_unit_contracts() {", 1)[1]
        cls.checker = compile(
            function.split("<<'PY'\n", 1)[1].split("\nPY\n", 1)[0],
            "verify-ipc-continuity.sh:source-contracts",
            "exec",
        )
        cls.sources = {
            name: (ROOT / name).read_text() for name in (SUPERVISOR, TRANSPORT, DAEMON)
        }

    def run_checker(self, **replacements: str) -> str:
        sources = self.sources | replacements
        output = StringIO()
        with patch.object(Path, "read_text", autospec=True) as read_text:
            read_text.side_effect = lambda path, **_: sources[str(path)]
            with redirect_stdout(output):
                exec(self.checker, {})
        return output.getvalue()

    def test_actual_explicit_constructor_and_separate_cancellation_channel_pass(
        self,
    ) -> None:
        self.assertIn("pub fn new(job_ids:", self.sources[SUPERVISOR])
        self.assertIn("mpsc::unbounded_channel()", self.sources[SUPERVISOR])
        self.assertIn("bounded IPC source contracts valid", self.run_checker())

    def test_each_event_ingress_must_use_its_exact_bounded_capacity(self) -> None:
        for capacity in (
            "BITBAKE_RELIABLE_EVENT_CAPACITY",
            "BITBAKE_COSMETIC_EVENT_CAPACITY",
            "1",
        ):
            for replacement in ("mpsc::unbounded_channel()", "mpsc::channel(999)"):
                with self.subTest(capacity=capacity, replacement=replacement):
                    original = f"mpsc::channel({capacity})"
                    self.assertIn(original, self.sources[SUPERVISOR])
                    mutated = self.sources[SUPERVISOR].replace(original, replacement, 1)
                    with self.assertRaisesRegex(SystemExit, "bounded.*ingress"):
                        self.run_checker(**{SUPERVISOR: mutated})

    def test_missing_constructor_fails_with_an_actionable_diagnostic(self) -> None:
        mutated = self.sources[SUPERVISOR].replace("pub fn new(", "pub fn renamed(", 1)
        with self.assertRaisesRegex(SystemExit, "supervisor constructor"):
            self.run_checker(**{SUPERVISOR: mutated})

    def test_bounded_calls_outside_constructor_cannot_mask_unbounded_ingress(
        self,
    ) -> None:
        mutated = self.sources[SUPERVISOR].replace(
            "mpsc::channel(BITBAKE_RELIABLE_EVENT_CAPACITY)",
            "mpsc::unbounded_channel()",
            1,
        )
        mutated += "\nfn decoy() { mpsc::channel(BITBAKE_RELIABLE_EVENT_CAPACITY); }\n"
        with self.assertRaisesRegex(SystemExit, "bounded.*ingress"):
            self.run_checker(**{SUPERVISOR: mutated})

    def test_transport_and_slow_client_requirements_remain_enforced(self) -> None:
        for name, token, diagnostic in (
            (TRANSPORT, "pub fn is_readable", "bounded daemon transport"),
            (DAEMON, "slow_client_disconnects", "slow-client isolation"),
        ):
            with self.subTest(name=name):
                mutated = self.sources[name].replace(token, "removed_contract")
                with self.assertRaisesRegex(SystemExit, diagnostic):
                    self.run_checker(**{name: mutated})


if __name__ == "__main__":
    unittest.main()
