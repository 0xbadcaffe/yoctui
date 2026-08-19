#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# This is the normal push/PR gate. Keep it deterministic and network-free;
# fresh official checkouts belong exclusively to the scheduled/manual job.
cargo test --locked -p yoctui-model --all-features compatibility_
cargo test --locked -p yoctui-bitbake --all-features compatibility_
cargo test --locked -p yoctui-app --all-features compatibility_
cargo test --locked -p yoctui-ui --all-features compatibility_
python3 -m pytest bridge/tests -q -k compatibility

./scripts/test-compatibility-matrix.sh --evidence-only
./scripts/verify-live-compatibility.sh --evidence-only
./scripts/verify-compatibility.sh --structure-only

printf '%s\n' 'deterministic release compatibility gates passed'
