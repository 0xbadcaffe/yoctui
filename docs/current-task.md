# Current Task

**ID:** DAEMON-BUILD-DIR-COMPAT-001
**Title:** Initialize daemon compatibility from the selected build directory
**Status:** IN_PROGRESS

Carry explicit daemon build-directory selection into the foreground process,
recover persisted workspace selection when needed, and publish current BitBake
capabilities before platform inspection is authorized.

Verify with:

```bash
cargo test -p yoctui --bin yoctui daemon_build_directory
cargo test -p yoctui --bin yoctui daemon_compatibility
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
