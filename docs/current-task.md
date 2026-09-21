# Current Task

**ID:** REDUCE-TOOLS-PYTHON-001
**Title:** Decompose oversized Python verification tools
**Status:** NOT_STARTED

Dependency REDUCE-TOOLS-PERF-SHELL-001 is DONE. Split
`scripts/generate-raw-catalog.py`, `scripts/event-flood-harness.py`,
`scripts/measure-ipc-latency.py` and `scripts/capture-real-poky-performance.py`
into meaningful parsing, measurement and evidence modules of approximately 500
lines or less. Preserve their command-line contracts and failure behavior.

```bash
python3 -m pytest bridge/tests
python3 -m compileall -q scripts
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
