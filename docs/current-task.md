# Current Task

**ID:** MENUCONFIG-RESUME-001
**Title:** Restore menuconfig keyboard input and running session recovery
**Status:** IN_PROGRESS

Repair mode-aware PTY input and model-owned platform session resume/recovery.
Dependencies are complete. See the M88 UI/architecture contracts.

```bash
cargo test -p yoctui-model platform_menuconfig
cargo test -p yoctui --bin yoctui menuconfig
cargo test -p yoctui-app pty_screen
cargo test -p yoctui-protocol pty_screen
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
