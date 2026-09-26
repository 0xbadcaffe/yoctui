# Current Task

**ID:** HARDWARE-RELEASE-001
**Title:** Package and install the Hardware feature series
**Status:** IN_PROGRESS

Bump v0.1.234, run the focused Hardware validation and strict workspace Clippy,
build and install the optimized binary, restart the initialized Romulus daemon,
and perform a focused local Hardware smoke test. The full workspace suite remains
deferred until requested.

Verify with:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
