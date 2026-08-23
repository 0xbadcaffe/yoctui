#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

./scripts/verify-raw-security.sh
./scripts/verify-raw-mode-evidence.sh
./scripts/check-docs.sh
cargo test --workspace --all-features raw_security -q
cargo test --workspace --all-features raw_compatibility -q
cargo fmt --all --check

echo "Raw Mode completion checks passed"
