#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
test -s README.md
grep -Fq 'oe-init-build-env' README.md
grep -Fq 'export BUILDDIR="$POKY_DIR/build-yoctui"' README.md
grep -Fq 'source "$POKY_DIR/oe-init-build-env" "$BUILDDIR"' README.md
grep -Fq 'yoctui --backend bridge' README.md
echo "README quickstart command checks passed"
