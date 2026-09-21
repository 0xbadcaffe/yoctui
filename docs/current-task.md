# Current Task

**ID:** REDUCE-MODEL-001
**Title:** Finish responsibility-based modules and test folders in yoctui-model
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-TESTS-001 is DONE. Perform the final model audit: every
production and test source must remain at approximately 500 lines or less, all
test bodies must live in `src/tests`, public APIs and reducer behavior must be
preserved, and source-checker paths must remain valid.

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
