#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
output="$(cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" --headless)"
if [[ "$output" != *"headless inspection completed"* ]]; then
  printf '%s\n' 'headless bridge inspection did not complete' >&2
  exit 1
fi

inspect="$(cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" inspect)"
if [[ "$inspect" != *"build directory:"* ]]; then
  printf '%s\n' 'bridge inspection did not report a build directory' >&2
  exit 1
fi
