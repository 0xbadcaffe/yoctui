# Current Task

**ID:** REDUCE-MODEL-SECURITY-TESTING-001
**Title:** Decompose security and test workflow models
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-QA-001 is DONE. Split security.rs and testing.rs into
meaningful modules for typed requests, reports, findings, sessions, comparison
projections and state transitions. Preserve public APIs, authority rules and
reducer behavior while targeting approximately 500 lines per production source.

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
