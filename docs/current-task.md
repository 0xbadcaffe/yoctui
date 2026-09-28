# Current Task

**ID:** HARDWARE-PDF-FALLBACK-RELEASE-001
**Title:** Release readable Hardware PDF fallback
**Status:** IN_PROGRESS

Bump to v0.1.246, run focused strict checks, build and install the optimized
binary, validate the reported 71-page PDF with native graphics forced off,
restart the initialized Romulus daemon, commit, and push.

```bash
cargo fmt --all --check
cargo clippy -p yoctui-model --all-features -- -D warnings
cargo clippy -p yoctui-app --all-features -- -D warnings
cargo clippy -p yoctui-ui --all-features -- -D warnings
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```

Full workspace tests remain deferred at the user's request.
