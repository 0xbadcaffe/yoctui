#!/usr/bin/env python3
"""Real terminal/editor/persistence acceptance using an isolated fixture daemon.

This exercises native PTYs and filesystem writes, not a real BitBake build.
"""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from pty_acceptance import binary_path, isolated_environment, snapshot_fixture, start_terminal


def send(terminal, keys):
    os.write(terminal.master, keys)


def paste(terminal, text):
    send(terminal, b"\x1b[200~" + text.encode() + b"\x1b[201~")


def wait_line(terminal, text):
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        terminal.read()
        if any(segment.strip() == text for line in terminal.screen.text().splitlines() for segment in line.split("│")):
            return
    raise AssertionError(f"Missing terminal result {text!r}:\n{terminal.screen.text()}")


def open_workbench(terminal):
    send(terminal, b"\x1b[20~")  # F9, then choose the actual catalog command.
    terminal.wait_for("Command")
    paste(terminal, "Open Workbench")
    terminal.wait_for("Open Workbench")
    send(terminal, b"\r")
    terminal.wait_for("Workbench · Sessions")
    # Global navigation preserves the caller's pane focus; enter the workspace.
    if "h/l groups" in terminal.screen.text():
        send(terminal, b"\t")
        terminal.wait_for("Alt+m actions")


def main():
    live = "--live-daemon" in sys.argv
    capture = Path(sys.argv[sys.argv.index("--capture-dir") + 1]) if "--capture-dir" in sys.argv else None
    with tempfile.TemporaryDirectory(prefix="yoctui-workbench-") as tmp:
        directory = Path(tmp)
        environment = isolated_environment(tmp)
        build = None if live else snapshot_fixture(tmp, ROOT, environment)
        if live:
            environment["XDG_RUNTIME_DIR"] = os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")
        source = directory / "source"
        source.mkdir()
        script = source / "000-script.sh"
        script.write_text("#!/bin/sh\necho original_marker\n")
        for index in range(100):
            (source / f"file-{index:03}.txt").write_text(f"payload_{index}\n")
        browse_root = directory / "browse"
        browse_root.mkdir()
        extra = browse_root / "extra"
        extra.mkdir()
        draft = extra / "draft.txt"
        draft.write_text("draft_original\n")
        prefix = [binary_path(ROOT), "--backend", "process", "--build-dir", build or tmp, "daemon"]
        terminal = None
        if not live:
            subprocess.run(prefix + ["start"], env=environment, check=True, capture_output=True)
        try:
            terminal = start_terminal(ROOT, environment, 160, 50, "attach")
            terminal.dismiss_onboarding()
            open_workbench(terminal)
            send(terminal, b"n")
            terminal.wait_for("New workspace · name")
            paste(terminal, "Recipe lab")
            send(terminal, b"\r")
            terminal.wait_for("Add folder · browse filesystem")
            send(terminal, b"\x15")
            paste(terminal, str(source))
            send(terminal, b"\r")
            terminal.wait_for("000-script.sh")
            send(terminal, b"\r")
            terminal.wait_for("original_marker")
            paste(terminal, "# edited\n")
            terminal.wait_for("# edited")
            send(terminal, b"\x13")
            terminal.wait_for("Saved ")
            assert script.read_text().startswith("# edited\n")
            # A completed save notification is dismissed before the next action.
            if "Notice" in terminal.screen.text():
                send(terminal, b"\x1b")
                terminal.wait_for("Explorer", absent="Notice")
            send(terminal, b"\x1bf")
            send(terminal, b"\x1b[F")
            terminal.wait_for("file-099.txt")
            send(terminal, b"\x1b[H")
            terminal.wait_for("000-script.sh")
            send(terminal, b"\x1ba")
            terminal.wait_for("Add folder · browse filesystem")
            send(terminal, b"\x15")
            paste(terminal, str(browse_root))
            send(terminal, b"\t")
            terminal.wait_for("extra/")
            # This parent has exactly one child directory.
            send(terminal, b"\x1b[H")
            terminal.wait_for("▶ ./")
            send(terminal, b"\x1b[B")
            terminal.wait_for("▶ extra/")
            if capture is not None:
                capture.mkdir(parents=True, exist_ok=True)
                (capture / "folder-browser.cells").write_text(terminal.screen.cell_golden())
            send(terminal, b"\r")
            terminal.wait_for(str(extra))
            terminal.wait_for("0 folder(s)")
            send(terminal, b" ")
            terminal.wait_for("draft.txt")
            send(terminal, b"\r")
            terminal.wait_for("draft_original")
            paste(terminal, "unsaved_")
            terminal.wait_for("unsaved_▏draft_original")
            send(terminal, b"\x1bo")
            terminal.wait_for("Workbench · Sessions")
            terminal.wait_for("unsaved")
            send(terminal, b"\x1b")
            terminal.wait_for("h/l groups")
            terminal.finish()
            os.close(terminal.master)
            terminal = start_terminal(ROOT, environment, 160, 50, "attach")
            terminal.wait_for("Quit")
            open_workbench(terminal)
            send(terminal, b"\r")
            terminal.wait_for("Explorer")
            send(terminal, b"\x1be")
            terminal.wait_for("unsaved_▏draft_original")
            assert draft.read_text() == "draft_original\n", "recovery must not save source implicitly"
            if capture is not None:
                capture.mkdir(parents=True, exist_ok=True)
                (capture / "workbench.cells").write_text(terminal.screen.cell_golden())
                send(terminal, b"\x1bm")
                terminal.wait_for("Devtool & workspace actions")
                (capture / "workbench-menu.cells").write_text(terminal.screen.cell_golden())
                send(terminal, b"\x1b")
                terminal.wait_for("Explorer", absent="Devtool & workspace actions")
            if live:
                send(terminal, b"\x1bt")
                terminal.wait_for("Choose terminal destination")
                send(terminal, b"\r")
                terminal.wait_for("WRITER")
                send(terminal, b"printf 'WORKBENCH_SHELL_OK\\n'\r")
                wait_line(terminal, "WORKBENCH_SHELL_OK")
                send(terminal, b"\x1bz")
                terminal.wait_for("WRITER")
                send(terminal, b"exit\r")
                terminal.wait_for("Exited")
            send(terminal, b"\x1bo")
            terminal.wait_for("Workbench · Sessions")
            send(terminal, b"\x1b")
            terminal.wait_for("h/l groups")
            terminal.finish()
            print("Workbench PTY PASS: create, folder browser, multi-folder tree, end/home, edit, atomic save, dirty recovery/reopen")
            print("Real shell writer/exit: PASS" if live else "Shell validation skipped: fixture environment is not a verified Yocto build; use --live-daemon")
        finally:
            if terminal is not None:
                terminal.cleanup()
                try:
                    os.close(terminal.master)
                except OSError:
                    pass
            if not live:
                subprocess.run(prefix + ["stop"], env=environment, check=True, capture_output=True)


if __name__ == "__main__":
    main()
