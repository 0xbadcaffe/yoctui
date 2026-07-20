#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

git diff --exit-code
git diff --cached --exit-code
cargo metadata --no-deps --format-version 1 >/dev/null
cargo fmt --all --check
cargo test --workspace --all-features
python3 -m unittest discover bridge/tests
./scripts/check-obsolete-name.sh
