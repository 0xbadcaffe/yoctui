# Current Task

**ID:** REDUCE-CLI-RUNTIME-TESTS-001
**Title:** Move remaining CLI runtime tests into test folders
**Status:** NOT_STARTED

Dependency REDUCE-CLI-DAEMON-WORKFLOW-TESTS-001 is DONE. Move inline tests from
global_search.rs, internal_tracing.rs, pty_attach.rs, render_scheduler.rs and
telemetry_scheduler.rs into responsibility-named files under the CLI test
folder. Move source-root pty_workflow_tests.rs there too, then run the final CLI
source and test-placement audit.

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
