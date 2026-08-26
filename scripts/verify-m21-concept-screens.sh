#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

cargo test -p yoctui-ui concept_screen_contracts
python3 scripts/test-m21-concept-screen-verifier.py
./scripts/verify-m21-concept-screens.py
./scripts/verify-m21-concept-pack.py
