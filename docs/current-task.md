# Current Task

**ID:** REDUCE-BITBAKE-TESTS-001
**Title:** Move BitBake tests into responsibility folders
**Status:** NOT_STARTED

Dependency REDUCE-BITBAKE-REMAINDER-001 is DONE. Move inline BitBake unit-test
bodies and oversized existing test modules into descriptive files under
`crates/yoctui-bitbake/src/tests`. Preserve shared fixtures, platform gates,
test names and assertions, and keep each test source near 500 lines or less.

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
