# Current Task

**ID:** MENUCONFIG-FAILURE-ACK-RELEASE-001
**Title:** Release menuconfig failure recovery
**Status:** IN_PROGRESS

Bump to v0.1.238, build and install the optimized binary, restart the initialized
Romulus daemon, and live validate that failed U-Boot menuconfig cleanup does not
block a following Kernel menuconfig launch.

Verification:

```bash
cargo fmt --all --check
cargo clippy -p yoctui-bitbake --all-features -- -D warnings
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
