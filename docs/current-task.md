# Current Task

**ID:** HARDWARE-NATIVE-GRAPHICS-RELEASE-001
**Title:** Release native Hardware document presentation
**Status:** IN_PROGRESS

Build and install v0.1.244, validate the reported PDF in native and fallback
presentations, confirm `Esc` returns to the library so another document can be
added/opened, restart the initialized Romulus daemon, commit, and push. Do not
run the full workspace suite until the user requests it.

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
