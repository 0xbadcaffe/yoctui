# Current Task

**ID:** REDUCE-PROTOCOL-001
**Title:** Finish responsibility-based modules and test folders in yoctui-protocol
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-001 is DONE. Audit `yoctui-protocol`, split this parent
into atomic file or responsibility-family tasks, then reduce production and test
sources to approximately 500 lines with test bodies under descriptive test
folders. Preserve public APIs, assertions and platform gates.

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
