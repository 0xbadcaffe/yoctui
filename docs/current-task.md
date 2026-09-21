# Current Task

**ID:** REDUCE-PROTOCOL-TESTS-001
**Title:** Move protocol inline tests into responsibility folders
**Status:** NOT_STARTED

Dependency REDUCE-PROTOCOL-SUPPORT-001 is DONE. Move all eight remaining inline
`yoctui-protocol` test modules into descriptive files under `src/tests`, preserve
shared fixtures and assertions, and finish the complete source/test-placement
audit with every Rust source at approximately 500 lines or less.

```bash
cargo test -p yoctui-protocol --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
