#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
mkdir -p artifacts/profile
cargo build --release -p yoctui
{ time ./scripts/headless-workload.sh target/release/yoctui; } 2>&1 | tee artifacts/profile/summary.txt
