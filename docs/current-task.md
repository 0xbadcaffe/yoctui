# Current Task

**ID:** REDUCE-MODEL-TESTS-001
**Title:** Move model inline tests into responsibility folders
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-REMAINDER-001 is DONE. Move every remaining inline
`yoctui-model` test and the existing source-root test module into descriptive
files under `src/tests`, preserving shared fixtures and assertions. Finish with
a complete source/test-placement audit.

```bash
cargo test -p yoctui-model --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
