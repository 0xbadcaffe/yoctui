# Current Task

**ID:** HARDWARE-PDF-WHEEL-RELEASE-001
**Title:** Release PDF wheel page navigation
**Status:** IN_PROGRESS

Bump the workspace to v0.1.250, run the focused release checks, build and install
the optimized binary, restart the initialized Romulus daemon, and repeat the
live page-1-to-page-2 wheel validation with the release artifact.

```bash
cargo fmt --all --check
cargo clippy -p yoctui-app --all-features -- -D warnings
cargo clippy -p yoctui-ui --all-features -- -D warnings
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```
