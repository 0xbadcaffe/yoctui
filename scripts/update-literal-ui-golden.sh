#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

echo "Updating the reviewed 160x48 literal UI golden. Inspect the resulting diff."
YOCTUI_UPDATE_LITERAL_GOLDEN=1 cargo test -p yoctui-ui literal_reference_cell_and_style_golden
git diff -- crates/yoctui-ui/tests/golden/literal-reference-160x48.cells
