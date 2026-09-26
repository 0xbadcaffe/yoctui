# Current Task

**ID:** MENUCONFIG-INTERACTION-001
**Title:** Flush initial menuconfig screens and toggle keyboard ownership
**Status:** IN_PROGRESS

Publish the final typed PTY screen after a bounded quiet period even when no
further output arrives. Add a model-owned `Ctrl+G` foreground toggle for the
exact Kernel/U-Boot menuconfig session: foreground sends every other key to the
PTY, while hidden mode leaves the process and writer lease alive and restores
the platform workspace with a resume hint.

Verification:

```bash
cargo test -p yoctui-model platform_menuconfig
cargo test -p yoctui-app platform_menuconfig
cargo test -p yoctui --bin yoctui menuconfig
cargo test -p yoctui-ui menuconfig
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
