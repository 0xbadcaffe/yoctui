#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
cargo build -p yoctui >/dev/null
python3 - "$repo_root" <<'PY'
import json, os, sys, tempfile, time
root = sys.argv[1]
sys.path.insert(0, os.path.join(root, "scripts"))
from pty_acceptance import isolated_environment, start_terminal
reports = os.path.join(root, "artifacts", "release-quality", "performance")
os.makedirs(reports, exist_ok=True)
rows = []
for width, height in ((80, 24), (160, 48)):
    with tempfile.TemporaryDirectory(prefix="yoctui-perf-", dir="/tmp") as tmp:
        build = os.path.join(tmp, "build")
        os.mkdir(build)
        environment = isolated_environment(tmp)
        started = time.perf_counter()
        terminal = start_terminal(root, environment, width, height,
                                  "--backend", "process", "--build-dir", build, "--no-color")
        try:
            # Window title and setup clear/home escapes are not a rendered frame.
            terminal.wait_for("Esc dismiss")
            first_frame = time.perf_counter() - started
            terminal.dismiss_onboarding()
            terminal.finish()
            rows.append({"width": width, "height": height, "first_frame_seconds": first_frame,
                         "bytes": len(terminal.raw), "returncode": terminal.process.returncode})
        finally:
            terminal.cleanup()
            os.close(terminal.master)
with open(os.path.join(reports, "tui.json"), "w") as handle:
    json.dump({"budgets": {"first_frame_seconds": 8, "output_bytes": 262144}, "samples": rows}, handle, indent=2)
if any(row["first_frame_seconds"] > 8 or row["bytes"] > 262144 or row["returncode"] != 0 for row in rows):
    raise SystemExit(f"performance budget exceeded: {rows}")
print("TUI performance budgets passed")
PY
