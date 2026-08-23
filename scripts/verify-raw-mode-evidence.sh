#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
evidence="${YOCTUI_RAW_EVIDENCE_DIR:-$repo_root/artifacts/raw-live}/summary.json"
test -s "$evidence" || { echo "Raw live evidence is missing: $evidence" >&2; exit 2; }
evidence_dir="$(dirname "$evidence")"
test -s "$evidence_dir/raw-version.txt" || { echo 'Raw native argv evidence is missing' >&2; exit 2; }
test -s "$evidence_dir/raw-pty.txt" || { echo 'Raw PTY evidence is missing' >&2; exit 2; }
python3 - "$evidence" "$repo_root" <<'PY'
import json, pathlib, subprocess, sys
path, repo = map(pathlib.Path, sys.argv[1:])
data = json.loads(path.read_text())
required = {"schema", "kind", "source_commit", "build_dir", "bitbake_version", "release", "machine", "layer_count", "normal_build_events", "cancellation_events", "raw_native_argv", "raw_pty_argv"}
missing = sorted(required - data.keys())
if missing: raise SystemExit("Raw live evidence missing fields: " + ", ".join(missing))
if data["schema"] != 1 or data["kind"] != "raw-live": raise SystemExit("Raw live evidence schema/kind is invalid")
if subprocess.run(["git", "merge-base", "--is-ancestor", data["source_commit"], "HEAD"], cwd=repo, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode != 0: raise SystemExit("Raw live evidence source commit is not an ancestor of HEAD")
if "build_started" not in data["normal_build_events"] or "build_started" not in data["cancellation_events"]: raise SystemExit("Raw live evidence lacks build lifecycle events")
print(f"Raw live evidence valid: BitBake {data['bitbake_version']} on {data['machine']}")
PY
