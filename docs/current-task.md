# Current Task

**ID:** REDUCE-CLI-CLIENT-TRANSPORT-001
**Title:** Decompose daemon client transport
**Status:** NOT_STARTED

Dependency REDUCE-CLI-CLIENT-RUNTIME-001 is DONE. Split client_transport.rs
into named handshake, request/reply-correlation and event-polling modules
targeting approximately 500 lines each. Move its inline tests into the CLI test
folder without changing retry, deadline, framing or correlation behavior.

```bash
cargo test -p yoctui --all-features client_transport
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
