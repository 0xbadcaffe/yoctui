# Current Task

**ID:** REDUCE-TOOLS-001
**Title:** Split bridge and verification tooling into named modules and test folders
**Status:** NOT_STARTED

Dependency REDUCE-E2E-001 is DONE. Audit `bridge`, `bridge/tests`, repository
scripts and `crates/yoctui-bitbake/bridge`; split oversized source and test
files into meaningful modules of approximately 500 lines or less. Preserve
behavior, public APIs, assertions and platform gates.

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
