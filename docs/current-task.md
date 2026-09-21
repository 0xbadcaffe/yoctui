# Current Task

**ID:** REDUCE-CLI-001
**Title:** Finish responsibility-based modules and test folders in yoctui-cli
**Status:** NOT_STARTED

Dependency REDUCE-CLI-RUNTIME-TESTS-001 is DONE. Run the final CLI source,
test-placement and source-checker audit. Confirm every production source is
approximately 500 lines or less, inline tests are gone, source-checker paths
cover all extracted modules and all task and baseline verification commands
pass. Make only audit corrections required by those findings.

```bash
cargo test -p yoctui --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
