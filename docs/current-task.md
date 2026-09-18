# Current Task

**ID:** REDUCE-CLI-DAEMON-001
**Title:** Decompose daemon service loop and command routing
**Status:** NOT_STARTED

Dependency REDUCE-CLI-MAIN-001 is DONE. main.rs is 468 lines and its tests
are in src/tests. Next split daemon_server.rs (about 1,520 lines) into named
startup, background polling, client service and command-routing modules,
targeting approximately 500 lines per file. Preserve ordering, authority,
backpressure, cancellation, bounded readiness and every existing assertion.

Relevant files: crates/yoctui-cli/src/daemon_server.rs and extracted daemon
modules; IPC/performance source-contract scripts. Done requires behavior
preservation, baseline and focused verification, registry/status/architecture
updates, and a coherent commit. Continue with REDUCE-CLI-TUI-001 afterward.

```bash
cargo test -p yoctui --all-features daemon
python3 -m unittest scripts/test_ipc_source_contracts.py
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
