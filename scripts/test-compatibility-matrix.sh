#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
./scripts/test-fresh-poky.sh
test -s docs/compatibility.md
echo "compatibility matrix bounded smoke passed"
