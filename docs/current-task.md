# Current Task

**ID:** REDUCE-UI-001
**Title:** Finish responsibility-based modules and test folders in yoctui-ui
**Status:** NOT_STARTED

Dependency REDUCE-UI-TESTS-001 is DONE. Audit all `yoctui-ui` Rust sources,
confirm every production and test file is approximately 500 lines or less,
confirm every test body lives under `src/tests`, run the required verification
commands and record the final crate result. Preserve behavior, public APIs,
assertions and platform gates.

```bash
cargo test -p yoctui-ui --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
