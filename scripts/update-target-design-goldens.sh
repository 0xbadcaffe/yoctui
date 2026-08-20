#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

echo "Updating the four reviewed 160x50 target-design goldens. Inspect every resulting diff."
YOCTUI_UPDATE_TARGET_GOLDENS=1 cargo test -p yoctui-ui target_design_golden
git diff -- crates/yoctui-ui/tests/golden/target-*.cells
