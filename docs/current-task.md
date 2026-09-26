# Current Task

**ID:** DTB-DECOMPILE-DIALOG-001
**Title:** Choose and view device-tree decompile outputs
**Status:** IN_PROGRESS

Implement the model-owned DTB/DTBO decompile form, bounded save-directory
browser, checked-by-default automatic viewer, and exact successful-PTY
completion correlation for Kernel and U-Boot.

Verify with:

```bash
cargo test -p yoctui-model device_tree_decompile
cargo test -p yoctui-app dtc_decompile
cargo test -p yoctui-ui dtc_decompile
cargo test -p yoctui --bin yoctui dtc_decompile
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
