# Current Task

**ID:** REDUCE-CLI-CORE-TESTS-001
**Title:** Move CLI core lifecycle tests into test folders
**Status:** NOT_STARTED

Dependency REDUCE-CLI-DAEMON-PTY-001 is DONE. Move inline tests from
build_archive.rs, clone_operation.rs, daemon_job_ids.rs,
environment_operation.rs and environment_setup.rs into responsibility-named
files under the CLI test folder. Preserve every fixture and assertion.

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
