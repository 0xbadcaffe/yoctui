# Current Task

**ID:** REDUCE-BITBAKE-001
**Title:** Finish responsibility-based modules and test folders in yoctui-bitbake
**Status:** NOT_STARTED

Dependency REDUCE-PROTOCOL-001 is DONE. Audit `yoctui-bitbake`, split this
parent into atomic file or responsibility-family tasks, then reduce production
and test sources to approximately 500 lines with test bodies under descriptive
test folders. Preserve public APIs, assertions and platform gates.

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
