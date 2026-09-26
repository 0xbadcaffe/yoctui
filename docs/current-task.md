# Current Task

**ID:** DEVTOOL-EDITOR-VIEWPORT-001
**Title:** Repair complete Devtool file and document navigation
**Status:** IN_PROGRESS

Make the integrated Devtool workspace file tree follow selection through the
complete bounded inventory, make long-document navigation visibly follow the
Vim-style cursor, retain in-TUI editing, highlight every known language in the
tree and source view, and give editor panes distinct semantic borders/titles.
Add model/app/TestBackend coverage for the visible failure paths.

Verify with:

```bash
cargo test -p yoctui-model devtool_editor_viewport
cargo test -p yoctui-app devtool_editor_viewport
cargo test -p yoctui-ui devtool_editor_viewport
cargo test -p yoctui --all-features workspace_editor
cargo fmt --all --check
```
