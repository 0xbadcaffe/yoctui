# Current Task

**ID:** PLATFORM-PTY-RELEASE-001
**Title:** Package and install the platform inspection and menuconfig correction series
**Status:** IN_PROGRESS

Prepare v0.1.233, run the focused correction checks, build and install the
optimized binary, restart the daemon in the initialized Romulus environment,
and smoke-test provider inspection from a plain client plus embedded
menuconfig startup. Do not run the full workspace suite during the user's
manual bug pass.

Verify with:

```bash
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
