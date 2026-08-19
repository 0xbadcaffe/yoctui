#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
mkdir -p artifacts/profile
summary="$(mktemp)"
trap 'rm -f "$summary"' EXIT
frames="${YOCTUI_PROFILE_FRAMES:-6000}"
if [[ ! "$frames" =~ ^[1-9][0-9]*$ ]]; then
  printf '%s\n' 'YOCTUI_PROFILE_FRAMES must be a positive integer' >&2
  exit 2
fi
YOCTUI_PROFILE_FRAMES="$frames" \
  cargo bench -q -p yoctui --bench workbench_profile 2>&1 | tee "$summary"
if ! grep -Eq "^yoctui workbench profile: frames=$frames checksum=[0-9a-f]{16} elapsed_ms=[1-9][0-9]*$" "$summary"; then
  printf '%s\n' 'deterministic workbench profile did not complete' >&2
  exit 1
fi
mv "$summary" artifacts/profile/summary.txt
