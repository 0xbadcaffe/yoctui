# Current Task

**ID:** REDUCE-CLI-DAEMON-WORKFLOW-TESTS-001
**Title:** Move remaining daemon workflow tests into test folders
**Status:** NOT_STARTED

Dependency REDUCE-CLI-DAEMON-ADAPTER-TESTS-001 is DONE. Move inline tests from
daemon_qemu.rs, daemon_sdk.rs, daemon_security.rs, daemon_test.rs and
daemon_wic.rs into responsibility-named files under the CLI test folder.
Preserve typed identity and failure coverage and every assertion.

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
