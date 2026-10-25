#!/usr/bin/env python3
"""Native performance UI/persistence and real host telemetry with an isolated fixture daemon.

This does not run BitBake, build an image or establish a build-speed improvement.
"""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

from pty_acceptance import binary_path, isolated_environment, snapshot_fixture, start_terminal

ROOT = Path(__file__).resolve().parents[1]


def send(terminal, keys):
    os.write(terminal.master, keys)


def open_performance(terminal):
    send(terminal, b"\x1b[20~")  # F9 palette
    terminal.wait_for("Command")
    send(terminal, b"\x1b[200~Build performance\x1b[201~")
    terminal.wait_for("Build performance")
    send(terminal, b"\r")
    terminal.wait_for("Build performance · opt-in profile")
    send(terminal, b"\t")
    terminal.wait_for("b Balanced")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--capture-dir", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="yoctui-performance-pty-") as directory:
        environment = isolated_environment(directory)
        build = snapshot_fixture(directory, ROOT, environment)
        # Supply the fixture's actual source root so it represents a configured workspace.
        bridge = Path(directory) / "fixture-bridge.py"
        bridge.write_text(Path(environment["YOCTUI_BRIDGE_PATH"]).read_text().replace(
            '"source_dir": None', '"source_dir": os.path.dirname(os.getcwd())'))
        environment["YOCTUI_BRIDGE_PATH"] = str(bridge)
        config = Path(build) / "conf/local.conf"
        original = config.read_bytes()
        prefix = [binary_path(ROOT), "--backend", "process", "--build-dir", build, "daemon"]
        subprocess.run(prefix + ["start"], env=environment, check=True, capture_output=True)
        terminal = None
        try:
            terminal = start_terminal(ROOT, environment, 160, 50, "attach")
            terminal.dismiss_onboarding()
            open_performance(terminal)
            terminal.wait_for("Sample Unix ms:", timeout=20)
            terminal.wait_for("RAM total")
            terminal.wait_for("CPU: some")
            terminal.wait_for("Memory: some")
            send(terminal, b"b")
            terminal.wait_for("Next build: Balanced")
            send(terminal, b"2")
            terminal.wait_for("> 2 BB_PRESSURE_MAX_IO")
            send(terminal, b"\x1b[C")
            terminal.wait_for("IO=20000")
            send(terminal, b"\x1b[F")
            terminal.wait_for("Cache checks show configuration")
            bottom = terminal.screen.text()
            send(terminal, b"\x1b[A")
            terminal.wait_for("Use these observations")
            assert terminal.screen.text() != bottom, "scrolling upward must leave the final viewport"
            send(terminal, b"\x1b[H")
            terminal.wait_for("HOST RESOURCES")
            if args.capture_dir:
                args.capture_dir.mkdir(parents=True, exist_ok=True)
                (args.capture_dir / "performance-160x50.cells").write_text(terminal.screen.cell_golden())
                (args.capture_dir / "performance-160x50.txt").write_text(terminal.screen.text())
            assert config.read_bytes() == original, "UI profile selection changed local.conf"
            send(terminal, b"\x1b")
            terminal.wait_for("h/l groups")
            terminal.finish()
            os.close(terminal.master)
            terminal = start_terminal(ROOT, environment, 160, 50, "attach")
            terminal.wait_for("Quit")
            open_performance(terminal)
            terminal.wait_for("IO=20000")
            send(terminal, b"d")
            terminal.wait_for("Next build: Existing BitBake configuration")
            for width, height in [(100, 30), (80, 24), (160, 50)]:
                terminal.resize(width, height)
                terminal.wait_for("Build performance")
                send(terminal, b"\x1b[F")
                terminal.wait_for("Cache checks show configuration")
                send(terminal, b"\x1b[H")
                terminal.wait_for("HOST RESOURCES")
            assert config.read_bytes() == original
            send(terminal, b"\x1b")
            terminal.wait_for("h/l groups")
            terminal.finish()
            print("FIXTURE PASS: native navigation/focus/scroll/resize, opt-in profile, persistence, detach/reconnect and unchanged local.conf")
            print("REAL PASS: daemon-host RAM/swap/PSI sampling. Real BitBake build/speed comparison: not exercised.")
        finally:
            if terminal:
                terminal.cleanup()
                try:
                    os.close(terminal.master)
                except OSError:
                    pass
            subprocess.run(prefix + ["stop"], env=environment, check=True, capture_output=True)


if __name__ == "__main__":
    main()
