#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
binary="${1:-target/debug/yoctui}"
"$binary" --headless --backend bridge --build-dir "$repo_root"
