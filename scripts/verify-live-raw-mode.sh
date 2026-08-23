#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
if [[ "${YOCTUI_LIVE_RAW:-0}" != 1 ]]; then
  echo 'SKIP live Raw validation: set YOCTUI_LIVE_RAW=1 and YOCTUI_LIVE_BUILD_DIR=/path/to/build.'
  exit 0
fi
build_dir="${YOCTUI_LIVE_BUILD_DIR:-${BUILDDIR:-}}"
if [[ -z "$build_dir" || ! -d "$build_dir" ]]; then echo 'live Raw validation: build directory is required' >&2; exit 2; fi
build_dir="$(cd "$build_dir" && pwd -P)"
if [[ ! -f "$build_dir/conf/bblayers.conf" || ! -f "$build_dir/conf/local.conf" ]]; then echo "live Raw validation: missing conf files in $build_dir" >&2; exit 2; fi
out="${YOCTUI_RAW_EVIDENCE_DIR:-$repo_root/artifacts/raw-live}"
mkdir -p "$out"
set +u
if [[ -n "${YOCTUI_OE_INIT_BUILD_ENV:-}" && -f "$YOCTUI_OE_INIT_BUILD_ENV" ]]; then
  source "$YOCTUI_OE_INIT_BUILD_ENV" "$build_dir" >/dev/null
elif [[ -f "$build_dir/init-build-env" ]]; then
  source "$build_dir/init-build-env" >/dev/null
fi
set -u
bitbake --version >"$out/raw-version.txt"
python3 -c 'import os, pty, sys; pty.spawn(["bitbake", "--version"])' >"$out/raw-pty.txt"
python3 "$repo_root/scripts/live_bitbake_smoke.py" --bridge "$repo_root/crates/yoctui-bitbake/bridge/yoctui_bridge.py" --build-dir "$build_dir" --target "${YOCTUI_LIVE_RAW_TARGET:-base-files}" --task "${YOCTUI_LIVE_RAW_TASK:-listtasks}" --cancel-target "${YOCTUI_LIVE_RAW_CANCEL_TARGET:-core-image-minimal}" --timeout "${YOCTUI_LIVE_TIMEOUT:-300}" >"$out/summary.json.tmp"
python3 - "$out/summary.json.tmp" "$out/summary.json" "$repo_root" "$build_dir" <<'PY'
import json, pathlib, subprocess, sys
source, destination, repo, build = map(pathlib.Path, sys.argv[1:])
data = json.loads(source.read_text())
data.update({"schema": 1, "kind": "raw-live", "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(), "build_dir": str(build), "raw_native_argv": "bitbake --version", "raw_pty_argv": "bitbake --version"})
destination.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
source.unlink()
PY
echo "live Raw validation evidence written: $out/summary.json"
