# Current Task

**ID:** REDUCE-APP-TESTS-001
**Title:** Move application tests into responsibility folders
**Status:** NOT_STARTED

Dependency REDUCE-APP-DAEMON-JOBS-001 is DONE. Move inline application unit-test
bodies and oversized existing test sources into descriptive files under
`crates/yoctui-app/src/tests`. Preserve shared fixtures, platform gates, test
names and assertions, and keep each test source near 500 lines or less.

```bash
cargo test -p yoctui-app --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
