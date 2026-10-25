#!/usr/bin/env python3
"""Native cache UI against an isolated fixture daemon; no BitBake builds or speed claims."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile
import time

from pty_acceptance import binary_path, isolated_environment, snapshot_fixture, start_terminal

ROOT = Path(__file__).resolve().parents[1]


def send(terminal, keys):
    os.write(terminal.master, keys)


def open_cache(terminal, menu=False):
    if menu:
        send(terminal, b"\x1b[24~")
        terminal.wait_for("Application Menu")
        send(terminal, b"\x1b[C")
        terminal.wait_for("Sstate cache")
        send(terminal, b"sst")
        terminal.wait_for("Jump: sst_")
        send(terminal, b"\r")
        terminal.wait_for("Sstate cache performance")
        send(terminal, b"\t")
        terminal.wait_for("e Configure")
        return
    send(terminal, b"\x1b[20~")
    terminal.wait_for("Command")
    send(terminal, b"\x1b[200~Sstate cache\x1b[201~")
    terminal.wait_for("Sstate cache")
    send(terminal, b"\r")
    terminal.wait_for("Sstate cache performance")
    send(terminal, b"\t")
    terminal.wait_for("e Configure")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--capture-dir", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="yoctui-cache-pty-") as directory:
        environment = isolated_environment(directory)
        build = snapshot_fixture(directory, ROOT, environment)
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
            open_cache(terminal, menu=True)
            terminal.wait_for("SSTATE REUSE")
            send(terminal, b"e")
            terminal.wait_for("Sstate cache settings")
            send(terminal, b"i")
            terminal.wait_for("INSERT")
            send(terminal, b"\x1b[200~https://cache.example.org/sstate\x1b[201~")
            terminal.wait_for("https://cache.example.org/sstate")
            send(terminal, b"\x1b")
            terminal.wait_for("NORMAL")
            send(terminal, b"\r")
            terminal.wait_for("Review sstate cache profile")
            terminal.wait_for("Mirror: https://cache.example.org/sstate")
            send(terminal, b"\r")
            terminal.wait_for("mirror configured", absent="Review sstate cache profile")
            terminal.resize(100, 30)
            terminal.wait_for("Sstate cache performance")
            send(terminal, b"\x1b[F")
            terminal.wait_for("p returns from the tools")
            bottom = terminal.screen.text()
            send(terminal, b"\x1b[A")
            deadline=time.monotonic()+8
            while terminal.screen.text()==bottom and time.monotonic()<deadline:
                terminal.read()
            assert terminal.screen.text()!=bottom, "scroll up must change the final viewport"
            send(terminal, b"\x1b[H")
            terminal.wait_for("SSTATE REUSE")
            terminal.resize(160, 50)
            terminal.wait_for("SSTATE REUSE")
            if args.capture_dir:
                args.capture_dir.mkdir(parents=True, exist_ok=True)
                (args.capture_dir / "sstate-160x50.cells").write_text(terminal.screen.cell_golden())
                (args.capture_dir / "sstate-160x50.txt").write_text(terminal.screen.text())
            assert config.read_bytes() == original
            send(terminal, b"\x1b")
            terminal.wait_for("p Cache performance")
            send(terminal, b"\x1b")
            terminal.wait_for("h/l groups")
            terminal.finish()
            os.close(terminal.master)
            terminal = start_terminal(ROOT, environment, 160, 50, "attach")
            terminal.wait_for("Quit")
            open_cache(terminal)
            terminal.wait_for("mirror configured")
            for width, height in [(100, 30), (80, 24), (160, 50)]:
                terminal.resize(width, height)
                terminal.wait_for("Sstate cache performance")
                send(terminal, b"\x1b[F")
                terminal.wait_for("p returns from the tools")
                send(terminal, b"\x1b[H")
                terminal.wait_for("SSTATE REUSE")
            send(terminal, b"u")
            terminal.wait_for("Sstate: existing BitBake configuration")
            assert config.read_bytes() == original
            send(terminal, b"t\x1b")
            terminal.wait_for("h/l groups")
            terminal.finish()
            print("FIXTURE PASS: native cache UI/paste/review/persistence/reconnect/scroll/resize, original tools preserved and local.conf unchanged")
            print("Live cache hits and image build-speed comparison: not exercised.")
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
