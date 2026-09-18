# Current Task

**ID:** REDUCE-CLI-MAIN-001
**Title:** Extract CLI entry-point responsibilities and move its inline tests
**Status:** NOT_STARTED

User request: review all source files, beginning with main.rs, target roughly
500 lines per file, use meaningful module names, and move tests into test folders.
The initial inventory finds main.rs at 23,100 lines. Extract configuration,
telemetry, commands, workflow coordinators and event publication into real Rust
modules. Keep main.rs as startup/module wiring; retain every test assertion in
named test modules. Oversized runtime loops receive explicit follow-up tasks.

Dependencies: none. Relevant files: crates/yoctui-cli/src/main.rs and extracted
CLI modules/tests; source-inspection scripts that refer to moved definitions.
Definition of done: main.rs is approximately 500 lines or fewer, test bodies are
in test folders, behavior/public APIs are preserved, verification passes, and
registry/status/architecture records and code are committed.

Verification:
```bash
cargo fmt --all --check
cargo test -p yoctui --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

The unrelated M67-LIVE-EVIDENCE-001 remains BLOCKED on a genuine current-source
real-Poky capture. Historical performance evidence and user captures stay intact.
