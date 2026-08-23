#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

# Raw execution must never route through a shell or interpolate a command
# string. Keep this check intentionally narrow so unrelated, explicitly typed
# PTY shell workflows are not mistaken for Raw command execution.
if rg -n 'Command::new\(("(sh|bash|zsh|fish)"|'"'"'(sh|bash|zsh|fish)'"'"')\)' \
  crates/yoctui-model/src/raw_mode.rs crates/yoctui-app/src/lib.rs; then
  echo "Raw security check failed: shell launcher in Raw crates" >&2
  exit 1
fi

cargo test --workspace --all-features raw_argv_rejects_every_documented_operator -q
echo "raw security verification passed"
