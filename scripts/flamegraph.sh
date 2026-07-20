#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
command -v cargo-flamegraph >/dev/null || { printf '%s\n' 'cargo-flamegraph is required; install it with cargo install flamegraph' >&2; exit 2; }
mkdir -p artifacts/flamegraph
cargo flamegraph --output artifacts/flamegraph/yoctui.svg --bin yoctui -- --headless --backend bridge --build-dir "$repo_root"
