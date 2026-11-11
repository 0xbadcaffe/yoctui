#!/usr/bin/env python3
"""Real UI/daemon switching with fixture BitBake metadata, isolated XDG roots and owned work."""
import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile

from pty_acceptance import binary_path, isolated_environment, snapshot_fixture, start_terminal

ROOT = Path(__file__).resolve().parents[1]


def send(terminal, keys):
    os.write(terminal.master, keys)


def environment_screen(terminal):
    send(terminal, b"\x1b[20~")
    terminal.wait_for("Commands")
    send(terminal, b"\x1b[200~Open Build Environment\x1b[201~")
    terminal.wait_for("Open Build Environment")
    send(terminal, b"\r")
    terminal.wait_for("Enter/e Configure paths")
    send(terminal, b"\t")


def set_paths(terminal, paths):
    environment_screen(terminal)
    send(terminal, b"e")
    terminal.wait_for("Configure build environment")
    for index, path in enumerate(paths):
        if index:
            send(terminal, b"\t")
        send(terminal, b"e")
        terminal.wait_for("Enter accept value")
        send(terminal, b"\x1b[200~" + str(path).encode() + b"\x1b[201~")
        terminal.wait_for(path.name)
        send(terminal, b"\r")
        terminal.wait_for("s save profile", absent="Enter accept value")
        terminal.wait_for(str(path))
    send(terminal, b"s")
    terminal.wait_for("Load build environment")


def record(environment):
    paths = list(Path(environment["XDG_RUNTIME_DIR"]).rglob("daemon.json"))
    assert len(paths) == 1, paths
    return json.loads(paths[0].read_text())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--capture-dir", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="yoctui-env-reconnect-") as directory:
        root = Path(directory)
        environment = isolated_environment(directory)
        old_build = Path(snapshot_fixture(directory, ROOT, environment))
        original = (old_build / "conf/local.conf").read_bytes()
        source = root / "source-b"
        source.mkdir()
        new_build = root / "new-build-b"
        marker = source / "initialized"
        selection = source / "recipe"
        selection.write_text("core-image-loaded")
        script = source / "custom-init.sh"
        script.write_text(
            'export BUILDDIR="$1"\nmkdir -p "$1/conf"\n'
            'printf \'MACHINE = "fixture-machine"\\n\' > "$1/conf/local.conf"\n'
            'printf \'BBLAYERS = ""\\n\' > "$1/conf/bblayers.conf"\n'
            f"printf initialized\\\\n >> {shlex.quote(str(marker))}\n"
            f"export YOCTUI_TEST_SOURCE={shlex.quote(str(source))}\n"
            f"export YOCTUI_TEST_RECIPE=$(cat {shlex.quote(str(selection))})\n"
            'cd "$BUILDDIR"\n'
        )
        # This deliberately readable, non-executable vendor wrapper is sourced.
        script.chmod(0o600)
        bridge = root / "fixture-bridge.py"
        text = Path(environment["YOCTUI_BRIDGE_PATH"]).read_text()
        text = text.replace('"source_dir": None', '"source_dir": os.environ.get("YOCTUI_TEST_SOURCE", os.path.dirname(os.getcwd()))')
        recipe = '{"name": os.environ.get("YOCTUI_TEST_RECIPE", "core-image-old"), "version": "1.0", "layer": "fixture"}'
        text = text.replace('"recipes": []', f'"recipes": [{recipe}]')
        text = text.replace('"variables": {}', '"variables": {"MACHINE": "fixture-machine"}')
        bridge.write_text(text)
        environment["YOCTUI_BRIDGE_PATH"] = str(bridge)
        binary = binary_path(ROOT)
        prefix = [binary, "--backend", "process", "--build-dir", str(old_build), "daemon"]
        subprocess.run(prefix + ["start"], env=environment, check=True, capture_output=True)
        old_identity = record(environment)
        terminal = None
        try:
            terminal = start_terminal(ROOT, environment, 180, 52, "attach")
            terminal.dismiss_onboarding()
            set_paths(terminal, [source, new_build, script])
            assert not marker.exists(), "Save/review sourced the script before confirmation"
            assert not new_build.exists(), "Review created the build directory"
            send(terminal, b"\r")
            terminal.wait_for("Loaded environment", timeout=30)
            terminal.wait_for(str(new_build))
            first_identity = record(environment)
            assert first_identity["daemon_instance_id"] != old_identity["daemon_instance_id"]
            assert marker.exists()
            send(terminal, b"\x1b[18~")  # F7 recipes
            terminal.wait_for("core-image-loaded", absent="core-image-old")
            if args.capture_dir:
                args.capture_dir.mkdir(parents=True, exist_ok=True)
                (args.capture_dir / "loaded-recipes.txt").write_text(terminal.screen.text())
            # Reopening must retain the exact verified custom initializer.
            send(terminal,b"\x1b")
            terminal.wait_for("h/l groups")
            terminal.finish()
            os.close(terminal.master)
            terminal=start_terminal(ROOT,environment,180,52,"attach")
            terminal.wait_for("Quit")
            # An unchanged triple must still re-source and refresh changed metadata.
            selection.write_text("core-image-refreshed")
            environment_screen(terminal)
            send(terminal, b"\x1bv")
            terminal.wait_for("Load build environment")
            send(terminal, b"\r")
            terminal.wait_for("Loaded environment", timeout=30)
            second_identity = record(environment)
            assert second_identity["daemon_instance_id"] != first_identity["daemon_instance_id"]
            send(terminal, b"\x1b[18~")
            terminal.wait_for("core-image-refreshed", absent="core-image-loaded")
            script.write_text("echo EXPECTED_SETUP_FAILURE >&2\nreturn 23\n")
            environment_screen(terminal)
            send(terminal, b"\x1bv")
            terminal.wait_for("Load build environment")
            send(terminal, b"\r")
            terminal.wait_for("EXPECTED_SETUP_FAILURE", absent="Load build environment", timeout=20)
            assert record(environment)["daemon_instance_id"] == second_identity["daemon_instance_id"], "Failed initialization stopped the previous daemon"
            assert (old_build / "conf/local.conf").read_bytes() == original
            if args.capture_dir:
                (args.capture_dir / "failure-preserves-daemon.txt").write_text(terminal.screen.text())
                (args.capture_dir / "result.json").write_text(json.dumps({"status":"FIXTURE PASS", "scope":"real native UI, filesystem initialization, daemon replacement/metadata rebind; fixture BitBake metadata", "old_instance":old_identity["daemon_instance_id"],"loaded_instance":first_identity["daemon_instance_id"],"refreshed_instance":second_identity["daemon_instance_id"],"client_reopen_preserved_custom_initializer":True,"original_config_unchanged":True,"failure_preserved_daemon":True},indent=2)+"\n")
            terminal.unwind()
            terminal.finish()
            assert terminal.process.returncode==0, "client did not exit cleanly"
            print("FIXTURE PASS: save→review→initialize→daemon switch→fresh recipes; same-path refresh; custom script/new build; failure preserves previous daemon/config")
        finally:
            if terminal:
                terminal.cleanup()
                os.close(terminal.master)
            subprocess.run([binary, "daemon", "stop"], env=environment, capture_output=True, check=True)


if __name__ == "__main__":
    main()
