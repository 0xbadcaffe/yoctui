# Current Task

**ID:** REDUCE-APP-MAPPING-001
**Title:** Decompose application event mapping
**Status:** NOT_STARTED

Dependency REDUCE-APP-INPUT-001 is DONE. Split `raw_mapping.rs`,
`runner_events.rs` and `compatibility_mapping.rs` into meaningful responsibility
files of approximately 500 lines or less. Preserve typed model actions,
backend-event semantics and public APIs.

```bash
cargo test -p yoctui-app --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
