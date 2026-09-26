# Current Task

**ID:** MENUCONFIG-INTERACTION-RELEASE-001
**Title:** Release menuconfig presentation and input corrections
**Status:** IN_PROGRESS

Bump to v0.1.237, run the focused formatting and strict Clippy gates, build and
install the optimized binary, restart the initialized Romulus daemon, and live
validate immediate menu rendering, keyboard navigation, `Ctrl+G` leave/resume,
normal exit, and detached startup for Kernel and U-Boot.

Verification:

```bash
cargo fmt --all --check
cargo clippy -p yoctui-model --all-features -- -D warnings
cargo clippy -p yoctui-app --all-features -- -D warnings
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
