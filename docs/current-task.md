# Current Task

**ID:** MENUCONFIG-RESUME-RELEASE-001
**Title:** Release menuconfig input and recovery fixes
**Status:** IN_PROGRESS

Dependencies are DONE. Bump to v0.1.241, run focused strict checks, build and
install the optimized binary, validate live navigation and client reattachment,
record limitations, commit and push. Full workspace suite remains deferred.

```bash
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```
