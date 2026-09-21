# Current Task

**ID:** REDUCE-MODEL-APP-STATE-001
**Title:** Decompose application state and action ownership
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-COMPATIBILITY-001 is DONE. Split app_state.rs and
actions.rs into meaningful modules for aggregate state construction,
selection/accessors, action families and observed timing types. Preserve public
APIs and typed reducer behavior while targeting approximately 500 lines per
source.

```bash
cargo test -p yoctui-model --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
