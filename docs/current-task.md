# Current Task

**ID:** REDUCE-CLI-DAEMON-BITBAKE-001
**Title:** Decompose daemon BitBake execution
**Status:** NOT_STARTED

Dependency REDUCE-CLI-CLIENT-TRANSPORT-001 is DONE. Split daemon_bitbake.rs
into named ingress, build-lifecycle and cancellation modules targeting
approximately 500 lines each. Move its inline tests into the CLI test folder
without changing bounded-priority, wakeup, cancellation or process ownership
behavior.

```bash
cargo test -p yoctui --all-features daemon_bitbake
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
