# Current Task

**ID:** REDUCE-CLI-DAEMON-ADAPTER-TESTS-001
**Title:** Move primary daemon adapter tests into test folders
**Status:** NOT_STARTED

Dependency REDUCE-CLI-CORE-TESTS-001 is DONE. Move inline tests from
daemon_devtool.rs, daemon_maintenance.rs, daemon_metadata.rs and daemon_qa.rs
into responsibility-named files under the CLI test folder. Preserve fake
process, cancellation and authority fixtures and every assertion.

```bash
cargo test -p yoctui --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
