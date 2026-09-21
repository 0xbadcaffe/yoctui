# Current Task

**ID:** REDUCE-PROTOCOL-001
**Title:** Finish responsibility-based modules and test folders in yoctui-protocol
**Status:** NOT_STARTED

Dependency REDUCE-PROTOCOL-TESTS-001 is DONE. Perform the final protocol audit:
every production and test source must remain at approximately 500 lines or less,
all test bodies must live under `src/tests`, public wire APIs and serde shapes
must be preserved, and all source-checker paths must remain valid.

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
