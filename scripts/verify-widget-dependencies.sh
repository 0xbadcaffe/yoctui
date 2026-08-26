#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python3 scripts/third_party_compliance.py candidates
python3 scripts/test-widget-dependency-verifier.py
cargo build --workspace --all-features --locked --offline

printf 'widget dependency gate valid: candidate graph audited; workspace lock builds offline\n'
