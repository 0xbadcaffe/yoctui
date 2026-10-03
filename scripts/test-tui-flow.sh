#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import os, sys, tempfile
root = sys.argv[1]
sys.path.insert(0, os.path.join(root, "scripts"))
from pty_acceptance import isolated_environment, start_terminal
with tempfile.TemporaryDirectory(prefix="yoctui-flow-", dir="/tmp") as tmp:
    build = os.path.join(tmp, "build")
    os.mkdir(build)
    terminal = start_terminal(root, isolated_environment(tmp), 80, 24,
                              "--backend", "process", "--build-dir", build, "--no-color")
    try:
        terminal.dismiss_onboarding()
        os.write(terminal.master, b"\t\x1b[Zyrlt\x1b[Z?")
        # Require the real unsupported-size frame, then recover the wide shell.
        terminal.resize(40, 12)
        terminal.wait_for("Yoctui needs at least 80x24")
        terminal.resize(160, 48)
        terminal.wait_for("Quit", absent="needs at least 80x24")
        terminal.unwind()
        terminal.finish()
        assert b"yoctui" in terminal.raw.lower(), "missing product identity"
    finally:
        terminal.cleanup()
        os.close(terminal.master)
print("real PTY navigation flow passed")
PY
