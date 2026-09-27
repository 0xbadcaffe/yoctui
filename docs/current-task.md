# Current Task

**ID:** MENUCONFIG-LOADING-001
**Title:** Keep menuconfig preparation visible and retain startup errors
**Status:** IN_PROGRESS

The keyboard/recovery dependency is complete. Keep loading until ncurses is
ready and retain bounded BitBake startup diagnostics without competing for the
interactive PTY. Update spec, status, registry, and commit.

```bash
cargo test -p yoctui --bin yoctui menuconfig_relay
cargo test -p yoctui-model platform_menuconfig
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
