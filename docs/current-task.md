# Current Task

**ID:** REDUCE-TOOLS-001
**Title:** Split bridge and verification tooling into named modules and test folders
**Status:** NOT_STARTED

Dependency REDUCE-TOOLS-PYTHON-001 is DONE. Audit `bridge`, `bridge/tests`,
repository scripts and `crates/yoctui-bitbake/bridge`; confirm every relevant
source is approximately 500 lines or less, tests live in test folders and all
entry points retain their command-line behavior.

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
