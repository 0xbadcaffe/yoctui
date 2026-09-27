# Current Task

**ID:** HARDWARE-PDF-VIEWER-RELEASE-001
**Title:** Release Hardware viewer corrections
**Status:** IN_PROGRESS

The readable Hardware viewer implementation and focused checks pass. Bump to
v0.1.243, run release checks and the optimized build, install it, validate the
reported 71-page PDF at fit and zoom with Navigator access, restart the
initialized Romulus daemon, commit and push.

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

The blocked U-Boot provider validation and M67 real-Poky evidence remain
independent. Full workspace tests remain deferred until the user requests them.
