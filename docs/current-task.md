# Current Task

**ID:** REDUCE-MODEL-RAW-MODE-001
**Title:** Decompose raw command state and behavior
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-RAW-CATALOG-001 is DONE. Split the 7,476-line raw mode
state and behavior into meaningful modules for catalog types, argv parsing,
parameter validation, capability authority, preview construction, selectors,
favorites/history and execution state. Preserve public APIs and reducer
behavior while targeting approximately 500 lines per production source.

```bash
cargo test -p yoctui-model --all-features raw
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
