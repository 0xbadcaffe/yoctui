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
./scripts/check-docs.sh

cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings

./scripts/test-terminal.sh
./scripts/test-fuzz.sh
./scripts/test-stress.sh
./scripts/test-sanitizers.sh

cargo llvm-cov -p yoctui-model --all-features --fail-under-lines 80
cargo llvm-cov -p yoctui-protocol --all-features --fail-under-lines 80

cargo audit
cargo deny check

bridge_source="crates/yoctui-bitbake/bridge"
bridge_tests="bridge/tests"
"$python_tools/ruff" check "$bridge_source" "$bridge_tests"
"$python_tools/ruff" format --check "$bridge_source" "$bridge_tests"
"$python_tools/mypy" "$bridge_source" "$bridge_tests"
"$python_tools/pytest" "$bridge_tests" \
  --cov="$bridge_source" --cov-report=term-missing --cov-fail-under=75

./scripts/valgrind.sh
./scripts/profile-workload.sh
./scripts/flamegraph.sh

if [[ -x ./scripts/verify-live-bitbake.sh ]]; then
  ./scripts/verify-live-bitbake.sh
else
  printf 'missing required live BitBake verification script\n' >&2
  exit 1
fi
