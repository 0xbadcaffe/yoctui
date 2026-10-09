"""Exercise the actual shell gate's Python source checks, including mutations."""

from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
SUPERVISOR = "crates/yoctui-cli/src/daemon_bitbake.rs"
SUPERVISOR_CANCELLATION = "crates/yoctui-cli/src/daemon_bitbake/cancellation.rs"
SUPERVISOR_INGRESS = "crates/yoctui-cli/src/daemon_bitbake/ingress.rs"
SUPERVISOR_LIFECYCLE = "crates/yoctui-cli/src/daemon_bitbake/lifecycle.rs"
SUPERVISOR_NOTIFICATION = "crates/yoctui-cli/src/daemon_bitbake/notification.rs"
TRANSPORT = "crates/yoctui-protocol/src/daemon_ipc.rs"
TRANSPORT_CONNECTION = "crates/yoctui-protocol/src/daemon_ipc/connection.rs"
DAEMON = "crates/yoctui-cli/src/daemon_server.rs"
SCHEDULING = "crates/yoctui-cli/src/daemon_scheduling.rs"
CLIENT_REQUESTS = "crates/yoctui-cli/src/daemon_server/client_requests.rs"


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
            name: (ROOT / name).read_text()
            for name in (
                SUPERVISOR,
                SUPERVISOR_CANCELLATION,
                SUPERVISOR_INGRESS,
                SUPERVISOR_LIFECYCLE,
                SUPERVISOR_NOTIFICATION,
                TRANSPORT,
                DAEMON,
                SCHEDULING,
                CLIENT_REQUESTS,
            )
        }
        cls.sources.update({
            str(path.relative_to(ROOT)): path.read_text()
            for path in (ROOT / TRANSPORT).with_suffix("").rglob("*.rs")
        })

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
        self.assertIn("pub fn new(job_ids:", self.sources[SUPERVISOR_LIFECYCLE])
        self.assertIn("mpsc::unbounded_channel()", self.sources[SUPERVISOR_LIFECYCLE])
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
                    self.assertIn(original, self.sources[SUPERVISOR_LIFECYCLE])
                    mutated = self.sources[SUPERVISOR_LIFECYCLE].replace(
                        original, replacement, 1
                    )
                    with self.assertRaisesRegex(SystemExit, "bounded.*ingress"):
                        self.run_checker(**{SUPERVISOR_LIFECYCLE: mutated})

    def test_missing_constructor_fails_with_an_actionable_diagnostic(self) -> None:
        mutated = self.sources[SUPERVISOR_LIFECYCLE].replace(
            "pub fn new(", "pub fn renamed(", 1
        )
        with self.assertRaisesRegex(SystemExit, "supervisor constructor"):
            self.run_checker(**{SUPERVISOR_LIFECYCLE: mutated})

    def test_bounded_calls_outside_constructor_cannot_mask_unbounded_ingress(
        self,
    ) -> None:
        mutated = self.sources[SUPERVISOR_LIFECYCLE].replace(
            "mpsc::channel(BITBAKE_RELIABLE_EVENT_CAPACITY)",
            "mpsc::unbounded_channel()",
            1,
        )
        mutated += "\nfn decoy() { mpsc::channel(BITBAKE_RELIABLE_EVENT_CAPACITY); }\n"
        with self.assertRaisesRegex(SystemExit, "bounded.*ingress"):
            self.run_checker(**{SUPERVISOR_LIFECYCLE: mutated})

    def test_transport_and_slow_client_requirements_remain_enforced(self) -> None:
        for name, token, diagnostic in (
            (TRANSPORT_CONNECTION, "pub fn is_readable", "bounded daemon transport"),
            (TRANSPORT_CONNECTION, "pub fn flush_event_frame", "bounded daemon transport"),
            (TRANSPORT_CONNECTION, "libc::MSG_DONTWAIT", "bounded daemon transport"),
            (TRANSPORT_CONNECTION, "Duration::from_secs(5)", "bounded daemon transport"),
            (DAEMON, "match connection.flush_event_frame()", "slow-client isolation"),
            (DAEMON, "client_requests::service", "slow-client isolation"),
            (CLIENT_REQUESTS, "connection.is_readable()?", "slow-client isolation"),
            (DAEMON, "slow_client_disconnects", "slow-client isolation"),
        ):
            with self.subTest(name=name):
                self.assertIn(token, self.sources[name])
                mutated = self.sources[name].replace(token, "removed_contract")
                with self.assertRaisesRegex(SystemExit, diagnostic):
                    self.run_checker(**{name: mutated})

    def test_missing_transport_module_content_is_not_silently_skipped(self) -> None:
        with self.assertRaisesRegex(SystemExit, "bounded daemon transport"):
            self.run_checker(**{TRANSPORT_CONNECTION: ""})

    def test_control_after_fanout_fails_the_slow_client_contract(self) -> None:
        mutated = self.sources[DAEMON].replace("client_requests::service", "service_requests", 1)
        mutated += "\n// client_requests::service\n"
        with self.assertRaisesRegex(SystemExit, "slow-client isolation"):
            self.run_checker(**{DAEMON: mutated})


if __name__ == "__main__":
    unittest.main()
