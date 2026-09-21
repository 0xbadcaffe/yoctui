# Current Task

**ID:** REDUCE-TOOLS-BRIDGE-001
**Title:** Decompose the Python BitBake bridge
**Status:** NOT_STARTED

Dependency REDUCE-E2E-001 is DONE. Split
`crates/yoctui-bitbake/bridge/yoctui_bridge.py` into meaningful protocol,
process, command and event modules of approximately 500 lines or less. Preserve
the deployed entry point, wire behavior, public APIs and platform gates.

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
