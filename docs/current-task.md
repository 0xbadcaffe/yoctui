# Current Task

**ID:** REDUCE-PROTOCOL-DAEMON-STATE-001
**Title:** Decompose daemon messages snapshots and framing
**Status:** NOT_STARTED

Dependency REDUCE-PROTOCOL-DAEMON-RAW-001 is DONE. Split client and server
messages, commands, PTY types, snapshot state, journal reduction, errors,
negotiation and framing out of `daemon.rs` into meaningful sources of
approximately 500 lines or less. Preserve wire representation and event order.

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
