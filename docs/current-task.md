# Current Task

**ID:** REDUCE-TOOLS-BRIDGE-TESTS-001
**Title:** Split bridge tests by responsibility
**Status:** NOT_STARTED

Dependency REDUCE-TOOLS-BRIDGE-001 is DONE. Split
`bridge/tests/test_bridge.py` into meaningful protocol, command, process and
event test files of approximately 500 lines or less. Move shared fixtures into
test support while preserving all 53 bridge tests and their assertions.

```bash
python3 -m pytest bridge/tests
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
