#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
command -v valgrind >/dev/null || { printf '%s\n' 'valgrind is required; install it before profiling' >&2; exit 2; }
mkdir -p artifacts/valgrind
cargo build -p yoctui
valgrind --leak-check=full --show-leak-kinds=all --track-fds=yes --track-origins=yes --error-exitcode=1 --xml=yes --xml-file=artifacts/valgrind/report.xml ./scripts/headless-workload.sh target/debug/yoctui 2>&1 | tee artifacts/valgrind/summary.txt
