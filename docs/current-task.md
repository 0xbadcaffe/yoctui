# Current Task

**ID:** REDUCE-MODEL-MAINTENANCE-001
**Title:** Decompose maintenance model workflows
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-RAW-MODE-001 is DONE. Split the 4,024-line maintenance
model into meaningful modules for shared types, service, release and sstate
workflows, validation and state transitions. Preserve typed requests, public
APIs, authority rules and reducer behavior while targeting approximately 500
lines per production source.

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
