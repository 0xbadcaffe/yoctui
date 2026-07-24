#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python_tools="${YOCTUI_PYTHON_TOOLS:-$HOME/.local/bin}"

require() {
  if ! "$@" >/dev/null 2>&1; then
    printf 'required completion tool is unavailable: %s\n' "$*" >&2
    exit 2
  fi
}

# Product completeness comes first. Quality checks cannot substitute for missing features.
./scripts/verify-product-complete.sh

require cargo llvm-cov --version
require cargo audit --version
require cargo deny --version
require "$python_tools/ruff" --version
require "$python_tools/mypy" --version
require "$python_tools/pytest" --version
require cargo flamegraph --version

./scripts/check-checkout.sh
./scripts/verify-ui-spec.sh

cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings

cargo llvm-cov -p yoctui-model --all-features --fail-under-lines 80
cargo llvm-cov -p yoctui-protocol --all-features --fail-under-lines 80

cargo audit
cargo deny check

"$python_tools/ruff" check bridge
"$python_tools/ruff" format --check bridge
"$python_tools/mypy" bridge
"$python_tools/pytest" bridge/tests --cov=bridge --cov-report=term-missing --cov-fail-under=75

./scripts/valgrind.sh
./scripts/profile-workload.sh
./scripts/flamegraph.sh

if [[ -x ./scripts/verify-live-bitbake.sh ]]; then
  ./scripts/verify-live-bitbake.sh
else
  printf 'missing required live BitBake verification script\n' >&2
  exit 1
fi
