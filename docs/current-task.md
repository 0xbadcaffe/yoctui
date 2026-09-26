# Current Task

**ID:** DEVTOOL-EDITOR-SEARCH-001
**Title:** Add selected-file and workspace-scoped Devtool search
**Status:** IN_PROGRESS

Route Ctrl+F to selected-buffer search, Ctrl+Shift+F to the shared bounded regex
surface scoped to the exact absolute workspace root, and `/` to its unchanged
global build-content scope. Selecting a workspace hit must load it into the
integrated editor without losing recipe context. Preserve all containment,
symlink, byte, result, cancellation, and responsive-dialog bounds.

Verify with:

```bash
cargo test -p yoctui-model devtool_editor_search
cargo test -p yoctui-app devtool_editor_search
cargo test -p yoctui --all-features workspace_search
cargo test -p yoctui-ui devtool_editor_search
cargo fmt --all --check
```
