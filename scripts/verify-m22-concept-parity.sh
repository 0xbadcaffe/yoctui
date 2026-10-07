#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

./scripts/verify-design-contracts.sh
./scripts/verify-m21-concept-screens.py --fixtures-only
./scripts/render-m22-concept-screenshots.sh --check
python3 scripts/test-m22-concept-raster.py

printf 'M22 production fixture/raster checks passed: 6/6 scenarios; live acceptance not checked\n'
