# Current Task

**ID:** REDUCE-CLI-CLIENT-RUNTIME-001
**Title:** Decompose interactive daemon client runtime
**Status:** NOT_STARTED

Dependency REDUCE-CLI-MAINTENANCE-001 is DONE. Split client_runtime.rs into
named attach, replica-application, typed effect-routing and terminal-control
modules targeting approximately 500 lines each. Move its inline tests into the
CLI test folder without changing assertions, daemon authority, reconnect,
request correlation or platform behavior.

```bash
cargo test -p yoctui --all-features client_runtime
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
