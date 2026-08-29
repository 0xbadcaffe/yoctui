#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

./scripts/verify-roadmap.sh
./scripts/verify-m21-concept-screens.py
./scripts/render-m22-concept-screenshots.sh --check
python3 scripts/test-m22-concept-raster.py
python3 scripts/test-m22-live-design-gallery.py
./scripts/verify-live-m22-concept-evidence.sh
python3 scripts/test-live-m22-concept-evidence.py

printf 'M22 concept-to-live parity passed: 6/6 scenarios, no open gaps\n'
