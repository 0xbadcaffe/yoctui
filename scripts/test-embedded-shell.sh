#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
if [[ "${1:-}" == "--backend" ]]; then
  cargo test -p yoctui-shell pty_backend
else
  cargo test -p yoctui-e2e pty_harness
fi
echo "embedded shell PTY coverage passed"
