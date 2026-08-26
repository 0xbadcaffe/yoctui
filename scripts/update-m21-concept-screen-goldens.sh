#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

echo "Updating six reviewed real-Yoctui concept cell and text goldens. Inspect every diff."
YOCTUI_UPDATE_CONCEPT_GOLDENS=1 cargo test -p yoctui-ui concept_screen_contracts
git diff -- crates/yoctui-ui/tests/golden/concept-*.cells crates/yoctui-ui/tests/golden/concept-*.txt
