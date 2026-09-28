# Current Task

**ID:** HARDWARE-PDF-AUTO-XTERM-RELEASE-001
**Title:** Release automatic readable graphics-terminal handoff
**Status:** IN_PROGRESS

Bump the workspace to v0.1.249, run the focused release checks, build and install
the optimized binary, and restart the initialized Romulus daemon. Confirm that
ordinary `yoctui attach` hands itself to the readable graphics terminal.

```bash
cargo fmt --all --check
cargo clippy -p yoctui-ui --all-features -- -D warnings
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```
