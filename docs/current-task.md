# Current Task

**ID:** MENUCONFIG-PTY-RELEASE-001
**Title:** Release the embedded menuconfig PTY correction
**Status:** IN_PROGRESS

Bump v0.1.235, build and install the optimized binary, restart the initialized
Romulus daemon, and live-verify that menuconfig exclusively owns its PTY until
the operator exits normally. The full workspace suite remains deferred until
requested.

```bash
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
