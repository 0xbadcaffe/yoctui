# Current Task

**ID:** SAVED-ENV-LOAD-001
**Title:** Load a reviewed saved environment into the live daemon workbench
**Status:** IN_PROGRESS

Dependency SAVED-ENV-DAEMON-001 is DONE. Relevant files: typed saved-environment
state/reducer, history input/review/rendering, CLI background lifecycle worker,
daemon shutdown guard and build-bound adapter rebinding. Implement the approved
reviewed loading specification. No archived execution state becomes live and
active jobs/PTYs must prevent replacement. Definition of done includes focused
normal/error/stale/modal/active-work coverage, documentation, version bump,
commit/push and a source-bound optimized release binary. Full suite deferred.

```bash
cargo test -p yoctui-model saved_environment
cargo test -p yoctui-app saved_environment
cargo test -p yoctui-ui saved_environment
cargo test -p yoctui --bin yoctui saved_environment
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
./scripts/verify-roadmap.sh
python3 scripts/check-version-bump.py
cargo build --release -p yoctui --bin yoctui
```

Manual: load saved Romulus paths, attach fresh authority and keep the daemon
alive after closing the initiating client. After delivery restore M67 blocker.
