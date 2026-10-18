"""Real client/daemon PTYs with a deterministic, explicitly fake setup tool.

Proves wizard routing, review/cancel, terminal input, import and persistence,
not a real image build or upstream network availability.
"""

import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import struct
import subprocess
import tempfile
import termios
import time

from pty_acceptance import (
    TerminalAcceptance,
    become_session_leader,
    isolated_environment,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/yoctui")
    parser.add_argument(
        "--official-tool",
        help="Optionally test this upstream executable with sibling local OE-Core/BitBake sources; no network or image build",
    )
    args = parser.parse_args()
    binary = str(Path(args.binary).resolve())
    with tempfile.TemporaryDirectory(prefix="yoctui-setup-pty-") as directory:
        root = Path(directory)
        environment = isolated_environment(directory)
        tool = root / "bitbake-setup"
        # Never calls BitBake or touches the developer's daemon/build/source trees.
        tool.write_text(
            "#!/usr/bin/env python3\n"
            "import pathlib, sys\n"
            "a = sys.argv[1:]\n"
            "settings = {}\n"
            "while a and a[0] == '--setting':\n"
            "    settings[a[2]] = a[3]; a = a[4:]\n"
            "assert a[:2] == ['init', '--no-init-vscode']\n"
            "name = a[a.index('--setup-dir-name') + 1]\n"
            "top = pathlib.Path(settings['top-dir-prefix']) / settings['top-dir-name']\n"
            "machine = input('Choose fixture machine: ')\n"
            "assert machine == 'qemux86-64', repr(machine)\n"
            "build = top / name / 'build'\n"
            "(build / 'conf').mkdir(parents=True)\n"
            "(build / 'conf/local.conf').write_text('MACHINE = \\\"qemux86-64\\\"\\n')\n"
            "(build / 'conf/bblayers.conf').write_text('BBLAYERS = \\\"\\\"\\n')\n"
            "(build / 'init-build-env').write_text('# deterministic fixture only\\n')\n"
            "print('SETUP_FIXTURE_COMPLETE', flush=True)\n"
        )
        tool.chmod(0o700)
        template = None
        if args.official_tool:
            tool = Path(args.official_tool).resolve()
            bitbake = tool.parent.parent
            oe_core = bitbake.parent / "openembedded-core"
            assert (oe_core / "scripts/oe-setup-build").is_file(), oe_core
            template = root / "local.conf.json"
            template.write_text(
                json.dumps(
                    {
                        "description": "Yoctui local upstream-tool validation",
                        "version": "1.0",
                        "sources": {
                            "bitbake": {"local": {"path": str(bitbake)}},
                            "openembedded-core": {"local": {"path": str(oe_core)}},
                        },
                        "bitbake-setup": {
                            "configurations": [
                                {
                                    "name": "local",
                                    "description": "Local sources; no network or build",
                                    "bb-layers": ["openembedded-core/meta"],
                                }
                            ]
                        },
                    }
                )
            )
        daemon = subprocess.run(
            [binary, "daemon", "start"],
            env=environment,
            cwd=root,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        assert daemon.returncode == 0, daemon.stderr
        terminal = None
        try:
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
            process = subprocess.Popen(
                [binary, "attach"],
                stdin=slave,
                stdout=slave,
                stderr=slave,
                env=environment,
                cwd=root,
                preexec_fn=become_session_leader,
            )
            os.close(slave)
            terminal = TerminalAcceptance(master, process, bytearray(), 100, 30)

            def press(key, expected, absent=None):
                os.write(master, key)
                terminal.wait_for(expected, absent=absent, timeout=15)

            def environment_view():
                press(b"\x1bOS", "Build Overview")  # F4 leaves terminal input routing
                press(b"\x10", "Command Palette")
                press(b"Open Build Environment", "Open Build Environment")
                press(b"\r", "Build environment", "Command Palette")
                if "h/l groups" in terminal.screen.text():
                    press(b"\r", "2 setups")

            def field(value):
                press(b"\r", "Enter accept field")
                press(b"\x1b[200~" + value.encode() + b"\x1b[201~", value[-25:])
                press(b"\r", "s review/save", "Enter accept field")

            terminal.dismiss_onboarding()
            environment_view()
            press(b"w", "New BitBake setup")
            field(str(tool))
            press(b"\t", "> Top directory")
            top = root / "setups"
            field(str(top))
            press(b"\t", "> Setup name")
            field("demo")
            if template:
                press(b"\t", "> Template")
                field(str(template))
            press(b"s", "Review new BitBake setup")
            assert not top.exists(), "review must not start the tool"
            press(b"\x1b", "s review/save", "Review new BitBake setup")
            assert not top.exists(), "cancel must not start the tool"
            press(b"s", "Review new BitBake setup")
            if template:
                press(
                    b"\r",
                    "Proceed? (y/N)",
                    "Review new BitBake setup",
                )
                if "· WRITER" not in terminal.screen.text():
                    press(b"o", "· WRITER")
                os.write(master, b"y\r")
                terminal.wait_for("Continue? (y/N)")
                os.write(master, b"y\r")
            else:
                press(b"\r", "Choose fixture machine:", "Review new BitBake setup")
                if "· WRITER" not in terminal.screen.text():
                    press(b"o", "· WRITER")
                os.write(master, b"qemux86-64\r")
            destination = top / "demo/build/conf/local.conf"
            deadline = time.monotonic() + 15
            while not destination.exists() and time.monotonic() < deadline:
                terminal.read()
            assert destination.exists(), terminal.screen.text()
            # Completed terminals intentionally disappear from the live tree.
            # Check the authoritative retained lifecycle, not an old ANSI frame.
            status = ""
            while time.monotonic() < deadline:
                status = subprocess.run(
                    [binary, "sessions"],
                    env=environment,
                    cwd=root,
                    capture_output=True,
                    text=True,
                    timeout=5,
                    check=True,
                ).stdout
                if "BitBake setup: demo" in status and "Exited" in status:
                    break
                terminal.read()
            assert "Exited" in status, status
            # Explicitly release terminal input before exercising global commands.
            os.write(master, b"\x1bo")
            terminal.wait_for("Terminal Sessions")
            environment_view()
            press(b"2", "Locally discovered setups")
            press(b"o", "Choose setup top directory")
            field(str(top))
            press(b"s", "> demo", "Choose setup top directory")
            terminal.wait_for("Locally discovered setups", absent="Loading directories")
            press(b"\r", "Configure build environment")
            assert str(top / "demo/build") in terminal.screen.text()
            press(b"\x1b", "> demo", "Configure build environment")
            terminal.finish()
            sessions = list(root.rglob("session.toml"))
            assert sessions, "client must persist normal-exit preferences"
            assert any(str(top) in path.read_text() for path in sessions)
            kind = (
                "official tool (local sources)" if template else "deterministic fixture"
            )
            print(
                f"BitBake setup {kind}: review/cancel, daemon PTY, discovery/import and root persistence passed"
            )
        finally:
            if terminal is not None:
                terminal.cleanup()
                os.close(terminal.master)
            subprocess.run(
                [binary, "daemon", "stop"],
                env=environment,
                cwd=root,
                capture_output=True,
                timeout=15,
                check=False,
            )


if __name__ == "__main__":
    main()
