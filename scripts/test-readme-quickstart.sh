#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
test -s README.md
grep -Fq 'oe-init-build-env' README.md
grep -Fq 'source "$YOCTO_DIR/oe-init-build-env"' README.md
grep -Fq 'yoctui --backend bridge' README.md
echo "README quickstart command checks passed"
