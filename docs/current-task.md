# Current Task

**ID:** DEVTOOL-WORKSPACE-SURFACE-001
**Title:** Add a first-class recipe-centered Devtool Workspace
**Status:** IN_PROGRESS

Implement a distinct `Screen::Devtool` reached by the Navigator's Devtool row.
It must reuse the authoritative recipe inventory, stable selected recipe
identity, typed Devtool status, and existing process owners while presenting the
ordered development workflow. It must not route the Devtool row to the general
Recipes screen.

Verify with:

```bash
cargo test -p yoctui-model devtool_workspace
cargo test -p yoctui-app devtool_workspace
cargo test -p yoctui-ui devtool_workspace
cargo fmt --all --check
```
