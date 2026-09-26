# Current Task

**ID:** ERRORS-WORKSPACE-RELEASE-001
**Title:** Package and install the Errors workspace improvement series
**Status:** IN_PROGRESS

The current/history viewer and resolved-history cleanup are complete. Package
the series as v0.1.232, run focused tests plus strict release checks, build and
install the optimized binary, restart the initialized Romulus daemon, commit,
and push normally.

Verify with:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
