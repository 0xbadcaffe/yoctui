"""Pure regression checks for hermetic, bounded real PTY acceptance helpers."""

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
from pty_acceptance import (
    TerminalAcceptance, binary_path, isolated_environment, snapshot_fixture,
)
from terminal_capture import Screen


class AcceptanceHelpers(unittest.TestCase):
    def executable(self, path):
        path.parent.mkdir(parents=True)
        path.write_text("#!/bin/sh\nexit 0\n")
        path.chmod(0o700)
        return str(path)

    def terminal(self):
        terminal = TerminalAcceptance.__new__(TerminalAcceptance)
        terminal.master = 42
        terminal.process = Mock(returncode=0)
        terminal.process.poll.return_value = 0
        terminal.raw = bytearray(b"\x1b[?1049h\x1b[?1049l")
        terminal.screen = Screen(80, 24)
        terminal.cursor_query_pending = b""
        terminal.read = Mock(return_value=False)
        return terminal

    def test_binary_uses_default_relative_and_absolute_cargo_targets(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            default = self.executable(root / "target/debug/yoctui")
            custom = self.executable(root / "custom/debug/yoctui")
            self.assertEqual(binary_path(root, {}), default)
            self.assertEqual(binary_path(root, {"CARGO_TARGET_DIR": "custom"}), custom)
            self.assertEqual(binary_path(root, {"CARGO_TARGET_DIR": str(root / "custom")}), custom)

    def test_missing_configured_binary_never_falls_back_to_stale_repo_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.executable(root / "target/debug/yoctui")
            with self.assertRaisesRegex(AssertionError, "built acceptance binary missing"):
                binary_path(root, {"CARGO_TARGET_DIR": "missing"})

    def test_nonexecutable_file_is_not_a_built_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "target/debug/yoctui"
            self.executable(path)
            path.chmod(0o600)
            with self.assertRaises(AssertionError):
                binary_path(tmp, {})

    def test_environment_is_private_nonmutating_and_disables_desktop_handoff(self):
        original = {"DISPLAY": ":0", "YOCTUI_BRIDGE_PATH": "/real/bridge",
                    "YOCTUI_BUILD_DIR": "/real/build", "BUILDDIR": "/real/build",
                    "BBPATH": "/real", "PYTHONHOME": "/real/python",
                    "XDG_RUNTIME_DIR": "/run/user/1000", "PATH": "/usr/bin",
                    "CARGO_TARGET_DIR": "custom"}
        before = original.copy()
        with tempfile.TemporaryDirectory() as tmp:
            isolated = isolated_environment(tmp, original)
            self.assertEqual(original, before)
            for key in ("BUILDDIR", "BBPATH", "YOCTUI_BUILD_DIR", "YOCTUI_BRIDGE_PATH", "PYTHONHOME"):
                self.assertNotIn(key, isolated)
            self.assertEqual(isolated["YOCTUI_TERMINAL_GRAPHICS"], "none")
            self.assertEqual(isolated["DISPLAY"], ":0")
            self.assertEqual(isolated["CARGO_TARGET_DIR"], "custom")
            self.assertEqual(isolated["TERM"], "xterm-256color")
            for variable in ("XDG_RUNTIME_DIR", "XDG_CONFIG_HOME", "XDG_STATE_HOME"):
                path = Path(isolated[variable])
                self.assertEqual(path.parent, Path(tmp))
                self.assertEqual(path.stat().st_mode & 0o777, 0o700)

    def test_snapshot_fixture_has_exact_initializer_and_no_build_execution(self):
        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as tmp:
            environment = isolated_environment(tmp)
            build = snapshot_fixture(tmp, root, environment)
            self.assertTrue((Path(build) / "conf/local.conf").is_file())
            self.assertTrue((Path(build) / "conf/bblayers.conf").is_file())
            self.assertEqual((Path(tmp) / "oe-init-build-env").stat().st_mode & 0o777, 0o700)
            result = subprocess.run(
                ["/bin/bash", "-c", '. "$1" "$2"; [ "$BUILDDIR" = "$2" ]',
                 "fixture", str(Path(tmp) / "oe-init-build-env"), build],
                env=environment, capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            version = subprocess.run([str(Path(tmp) / "bin/bitbake"), "--version"], capture_output=True)
            self.assertEqual(version.returncode, 0)
            forbidden = subprocess.run([str(Path(tmp) / "bin/bitbake"), "core-image-minimal"], capture_output=True)
            self.assertNotEqual(forbidden.returncode, 0)
            self.assertEqual(environment["PYTHON"], sys.executable)
            self.assertTrue(Path(environment["YOCTUI_BRIDGE_PATH"]).is_file())

    def test_cursor_reports_use_actual_composed_positions_in_order(self):
        terminal = self.terminal()
        with patch("pty_acceptance.os.write") as write:
            terminal.feed_output(b"\x1b[2;3HA\x1b[6nB\x1b[6n")
            self.assertEqual([call.args for call in write.call_args_list],
                             [(42, b"\x1b[2;4R"), (42, b"\x1b[2;5R")])

    def test_fragmented_cursor_report_has_bounded_pending_state(self):
        terminal = self.terminal()
        with patch("pty_acceptance.os.write") as write:
            terminal.feed_output(b"\x1b[2;3HA\x1b[6")
            self.assertEqual(terminal.cursor_query_pending, b"\x1b[6")
            write.assert_not_called()
            terminal.feed_output(b"nB")
            write.assert_called_once_with(42, b"\x1b[2;4R")
            self.assertEqual(terminal.cursor_query_pending, b"")

    def test_unrecognized_query_does_not_get_a_fabricated_cursor_report(self):
        terminal = self.terminal()
        with patch("pty_acceptance.os.write") as write:
            terminal.feed_output(b"\x1b[5n")
            write.assert_not_called()

    def test_title_escape_is_not_rendered_onboarding_readiness(self):
        terminal = self.terminal()
        terminal.screen.feed(b"\x1b]0;yoctui Esc dismiss\x07")
        terminal.process.poll.return_value = None
        with patch("pty_acceptance.time.monotonic", side_effect=[0, 1, 9]):
            with self.assertRaisesRegex(AssertionError, "PTY missing 'Esc dismiss'"):
                terminal.wait_for("Esc dismiss")

    def test_wait_for_requires_popup_absence_as_well_as_footer(self):
        terminal = self.terminal()
        terminal.screen = Mock()
        terminal.screen.text.side_effect = ["Quit Esc dismiss", "Quit"]
        terminal.process.poll.return_value = None
        with patch("pty_acceptance.time.monotonic", side_effect=[0, 1, 2]):
            terminal.wait_for("Quit", absent="Esc dismiss")
        self.assertEqual(terminal.read.call_count, 2)

    def test_successful_exit_requires_alternate_screen_restoration(self):
        terminal = self.terminal()
        terminal.raw = bytearray(b"\x1b[?1049h")
        with patch("pty_acceptance.os.write"), self.assertRaisesRegex(AssertionError, "not entered and restored"):
            terminal.finish()

    def test_visible_footer_does_not_return_while_frame_output_is_pending(self):
        terminal = self.terminal()
        terminal.screen.feed("Quit")
        terminal.read.side_effect = [True, False]
        terminal.process.poll.return_value = None
        with patch("pty_acceptance.time.monotonic", side_effect=[0, 1, 2]):
            terminal.wait_for("Quit")
        self.assertEqual(terminal.read.call_count, 2)

    def test_forced_cleanup_is_not_a_successful_exit(self):
        terminal = self.terminal()
        terminal.process.poll.return_value = None
        terminal.process.returncode = -9
        with patch("pty_acceptance.os.write"), patch("pty_acceptance.time.monotonic", side_effect=[0, 4]):
            with self.assertRaisesRegex(AssertionError, "did not exit cleanly: -9"):
                terminal.finish()
        terminal.process.kill.assert_called_once()


if __name__ == "__main__":
    unittest.main()
