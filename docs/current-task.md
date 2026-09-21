# Current Task

**ID:** REDUCE-BITBAKE-MAINTENANCE-001
**Title:** Decompose BitBake maintenance adapters
**Status:** NOT_STARTED

Dependency REDUCE-PROTOCOL-001 is DONE. Split the production responsibilities
in `maintenance_sstate.rs`, `maintenance_release.rs`, `maintenance_service.rs`
and `maintenance_optional.rs` into meaningful sources of approximately 500
lines or less. Preserve public APIs, process ownership and validation behavior.

```bash
cargo test -p yoctui-bitbake --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
