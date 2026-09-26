# Current Task

**ID:** MENUCONFIG-FAILURE-ACK-001
**Title:** Complete failed menuconfig wrappers without operator input
**Status:** IN_PROGRESS

Detect OpenEmbedded's exact menuconfig failure acknowledgement prompt across
bounded PTY output chunks, acknowledge it once through the daemon supervisor,
retain the diagnostic screen, and allow the session to publish its actual exit
instead of blocking later Kernel or U-Boot operations.

Verification:

```bash
cargo test -p yoctui --bin yoctui menuconfig_failure
cargo fmt --all --check
cargo clippy -p yoctui-bitbake --all-features -- -D warnings
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
