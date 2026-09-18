# Current Task

**ID:** REDUCE-CLI-TUI-001
**Title:** Decompose interactive runtime state input and effects
**Status:** NOT_STARTED

Dependencies REDUCE-CLI-MAIN-001 and REDUCE-CLI-DAEMON-001 are DONE.
main.rs is 472 lines; daemon orchestration and every extracted daemon module
are below 500 lines. Split interactive_runtime.rs into typed runtime state,
startup/shutdown, background polling, presentation and named input/effect
routing modules. Target approximately 500 lines per file. Preserve branch
order, outer-loop continuation, ownership, cancellation and render cadence.

Relevant files: crates/yoctui-cli/src/interactive_runtime.rs and its new
modules; scripts that inspect interactive sources. Keep existing assertions,
add focused regression coverage where needed, update architecture and task
records, and commit after verification. Continue with remaining CLI files.

```bash
cargo test -p yoctui --all-features
./scripts/test-terminal.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
