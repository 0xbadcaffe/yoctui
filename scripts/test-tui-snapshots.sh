#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, re, subprocess, sys, tempfile
root = sys.argv[1]
sys.path.insert(0, os.path.join(root, "scripts"))
from pty_acceptance import binary_path, isolated_environment, snapshot_fixture, start_terminal
artifact = os.path.join(root, "artifacts", "release-quality", "snapshots")
os.makedirs(artifact, exist_ok=True)
for width, height, name in ((80, 24, "narrow"), (100, 30, "medium"), (160, 48, "wide")):
    with tempfile.TemporaryDirectory(prefix="yoctui-snapshot-", dir="/tmp") as tmp:
        environment = isolated_environment(tmp)
        build = snapshot_fixture(tmp, root, environment)
        # Real daemon IPC against an explicitly initialized offline fixture;
        # this is not a claim of live BitBake or OpenBMC compatibility.
        daemon_prefix = [binary_path(root), "--backend", "process", "--build-dir", build, "daemon"]
        terminal = None
        started = False
        try:
            result = subprocess.run(daemon_prefix + ["start"], env=environment, capture_output=True)
            if result.returncode != 0:
                raise AssertionError(f"snapshot daemon startup failed: {result.stderr.decode()}")
            started = True
            terminal = start_terminal(root, environment, width, height,
                                      "--backend", "process", "--build-dir", build, "--no-color")
            terminal.dismiss_onboarding(workbench_timeout=.5)
            if name == "wide":
                os.write(terminal.master, b"\x1bOQ")
                terminal.wait_for("Tasks: not selected · All", timeout=2)
            normalized = terminal.screen.text()
            # Only historical timing/activity normalization; version stays exact.
            normalized = re.sub(r"(?<!\d)\d{2}:\d{2}(?::\d{2})?(?!\d)", "00:00", normalized)
            normalized = re.sub(r"[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏⠟⠯⠷⠾⠽⠻]", "⠋", normalized)
            with open(os.path.join(artifact, f"{name}.txt"), "w") as output:
                output.write(normalized[-32768:])
            if "yoctui" not in normalized.lower():
                raise AssertionError(f"snapshot missing product identity at {name}")
            if name == "wide":
                for anchor in ("Tasks: not selected · All", "F1 Help", "F12 Menu"):
                    if anchor not in normalized:
                        raise AssertionError(f"wide reference snapshot missing {anchor!r}")
                if "D:✓ Connected/Local" not in normalized:
                    raise AssertionError("wide snapshot must be attached to the actual fixture daemon")
            terminal.finish()
        finally:
            if terminal is not None:
                terminal.cleanup()
                os.close(terminal.master)
            if started:
                stopped = subprocess.run(daemon_prefix + ["stop"], env=environment, capture_output=True)
                if stopped.returncode != 0:
                    raise AssertionError(f"owned snapshot daemon cleanup failed: {stopped.stderr!r}")
print("PTY semantic snapshots passed")
PY
