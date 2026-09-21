# Current Task

**ID:** REDUCE-PROTOCOL-DAEMON-RAW-001
**Title:** Decompose daemon raw execution and compatibility wire types
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-001 is DONE. Split the raw execution, history, retained
output, compatibility identity, evidence and capability protocol families out of
`daemon.rs` into meaningful sources of approximately 500 lines or less. Preserve
serde representations, public exports, validation semantics and assertions.

```bash
cargo test -p yoctui-protocol --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
