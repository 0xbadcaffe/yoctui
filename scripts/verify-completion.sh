#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

require() {
  if ! "$@" >/dev/null 2>&1; then
    printf 'required completion tool is unavailable: %s\n' "$*" >&2
    exit 2
  fi
}

require cargo llvm-cov --version
require cargo audit --version
require cargo deny --version
require ruff --version
require mypy --version
require pytest --version
require cargo flamegraph --version

./scripts/check-checkout.sh
cargo llvm-cov -p yoctui-model --all-features --fail-under-lines 80
cargo llvm-cov -p yoctui-protocol --all-features --fail-under-lines 80
cargo audit
cargo deny check
ruff check bridge
ruff format --check bridge
mypy bridge
pytest bridge/tests
./scripts/valgrind.sh
./scripts/profile-workload.sh
./scripts/flamegraph.sh
