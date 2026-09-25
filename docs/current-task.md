# Current Task

**ID:** DEVTOOL-WORKSPACE-LOOP-001
**Title:** Connect start edit build and SSH/SCP deployment in the Devtool Workspace
**Status:** IN_PROGRESS

Connect the dedicated Devtool Workspace to the existing typed modify, source
editor, workspace shell, GitUI, exact recipe build, and deploy-target workflows.
Every dialog or job must retain the selected recipe and return to the Devtool
screen. Label deployment as Devtool's SSH/SCP transport and preview its exact
target without guessing a binary path.

Verify with:

```bash
cargo test -p yoctui-model devtool_workspace
cargo test -p yoctui-app devtool_workspace
cargo test -p yoctui-ui devtool_workspace
cargo fmt --all --check
```
