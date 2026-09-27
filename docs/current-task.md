# Current Task

**ID:** DAEMON-BUILD-DIR-COMPAT-RELEASE-001
**Title:** Release daemon build-directory compatibility correction
**Status:** IN_PROGRESS

Publish v0.1.240, install its optimized binary, restart the daemon with the
Romulus build directory, and confirm its live compatibility snapshot authorizes
BitBake variable and recipe metadata queries.

Verify with:

```bash
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```
