# Current Task

**ID:** DEVTOOL-EDITOR-GIT-001
**Title:** Show complete workspace Git state and direct GitUI access
**Status:** IN_PROGRESS

Report repository root, branch, upstream synchronization, and dirty state for
the exact recipe workspace in both the Devtool screen and integrated editor.
Expose GitUI directly from the integrated editor while preserving the existing
typed terminal-launch preview and exact workspace working directory.

Verify with:

```bash
cargo test -p yoctui-bitbake devtool_workspace_git
cargo test -p yoctui-model devtool_editor_git
cargo test -p yoctui-app devtool_editor_git
cargo test -p yoctui-ui devtool_editor_git
cargo fmt --all --check
```
