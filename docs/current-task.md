# Current Task

**ID:** PLATFORM-INSPECTION-SNAPSHOT-001
**Title:** Inspect U-Boot without a daemon capability snapshot
**Status:** IN_PROGRESS

Route Kernel and Firmware inspection as client-local orchestration because the
CLI worker initializes and owns its metadata backend. Preserve current
capability enforcement for menuconfig and builds. Reproduce absent-snapshot
U-Boot routing, verify the focused fix, update governance, and commit.

```bash
cargo test -p yoctui-model workspace_compatibility
cargo test -p yoctui-app platform_inspection
cargo test -p yoctui --bin yoctui platform_inspection
./scripts/verify-roadmap.sh
```
