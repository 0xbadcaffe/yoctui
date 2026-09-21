# Current Task

**ID:** REDUCE-TOOLS-PERF-SHELL-001
**Title:** Decompose the performance verification shell
**Status:** NOT_STARTED

Dependency REDUCE-TOOLS-BRIDGE-TESTS-001 is DONE. Split
`scripts/verify-performance.sh` into named shell modules of approximately 500
lines or less. Preserve its command-line contract, measurement orchestration,
evidence validation and failure behavior.

```bash
python3 -m pytest bridge/tests
bash -n scripts/verify-performance.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
