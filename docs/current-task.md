# Current Task

**ID:** DEVTOOL-WORKSPACE-PATCH-001
**Title:** Create and install recipe patches into a configured layer
**Status:** IN_PROGRESS

Add a typed configured-layer plan for
`devtool update-recipe --mode patch --append <layer> <recipe>`. The picker must
retain the exact recipe identity, list only absolute configured layers, validate
workspace eligibility and layer membership before execution, run through the
daemon-owned Devtool job, refresh status on success, and keep the existing
`devtool finish` publication path available.

Verify with:

```bash
cargo test -p yoctui-model devtool_patch
cargo test -p yoctui-bitbake devtool_patch
cargo test -p yoctui-app devtool_patch
cargo test -p yoctui-ui devtool_patch
cargo fmt --all --check
```
