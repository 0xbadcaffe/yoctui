# Current Task

**ID:** TERMINAL-DEC-GRAPHICS-RELEASE-001
**Title:** Release DEC Special Graphics terminal correction
**Status:** IN_PROGRESS

Bump v0.1.236, build and install the optimized binary, restart the initialized
Romulus daemon, and live-verify that Kernel menuconfig renders readable Unicode
borders without DEC source letters. The full workspace suite remains deferred
until requested.

```bash
cargo fmt --all --check
cargo clippy -p yoctui-model --all-features -- -D warnings
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
