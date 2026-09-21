# Current Task

**ID:** REDUCE-UTILS-001
**Title:** Finish responsibility-based modules and test folders in yoctui-utils
**Status:** NOT_STARTED

Dependency REDUCE-CLI-001 is DONE. Audit all Rust source in crates/yoctui-utils.
Target approximately 500 lines per production source file using meaningful
responsibility names, and move inline test bodies into responsibility-named
files under a test folder. Split the task into atomic child tasks first if the
implementation is too large for one coherent commit.

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
