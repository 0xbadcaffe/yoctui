# Current Task

**ID:** BACKEND-PROBE-RECOVERY-RELEASE-001
**Title:** Release backend capability recovery
**Status:** IN_PROGRESS

Implementation dependency is DONE; live recovery of the retained unknown
snapshot passed on Romulus. Bump/build/install v0.1.242, restart the initialized
daemon, verify current API authority and fresh-client Kernel inspection, record
results, commit and push. Full workspace tests remain deferred.

```bash
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```

Existing MENUCONFIG-UBOOT-LIVE-001 and M67-LIVE-EVIDENCE-001 remain externally blocked.
