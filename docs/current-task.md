# Current Task

**ID:** REDUCE-BITBAKE-REMAINDER-001
**Title:** Decompose remaining oversized BitBake sources
**Status:** NOT_STARTED

Dependency REDUCE-BITBAKE-BACKEND-001 is DONE. Split remaining production
sources above approximately 500 lines into meaningful responsibility files.
Preserve public APIs, platform gates and adapter behavior, then audit every
production source under `crates/yoctui-bitbake/src`.

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
