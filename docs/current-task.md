# Current Task

**ID:** HARDWARE-PDF-NATIVE-ONLY-RELEASE-001
**Title:** Release readable native PDF viewing
**Status:** IN_PROGRESS

Release v0.1.248, install it, validate the release in XTerm VT340, restart the
initialized Romulus daemon, and commit/push.

```bash
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```

Full workspace tests remain deferred at the user's request.
