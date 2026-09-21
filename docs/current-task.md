# Current Task

**ID:** REDUCE-BITBAKE-001
**Title:** Finish responsibility-based modules and test folders in yoctui-bitbake
**Status:** NOT_STARTED

Dependency REDUCE-BITBAKE-TESTS-001 is DONE. Audit every BitBake source and
test file for the approximately 500-line target and responsibility placement.
Verify public APIs, platform gates, test names and assertions remain preserved,
then close the parent decomposition task.

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
