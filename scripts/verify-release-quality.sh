#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
./scripts/test-tui-pty.sh
./scripts/test-tui-keymap.sh
./scripts/test-tui-flow.sh
./scripts/test-tui-snapshots.sh
./scripts/test-tui-performance.sh
./scripts/test-readme-quickstart.sh
./scripts/verify-utility-coverage.sh
./scripts/test-embedded-shell.sh
./scripts/verify-roadmap.sh
echo "release-quality deterministic gate passed"
