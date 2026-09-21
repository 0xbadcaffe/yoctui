# Current Task

**ID:** REDUCE-MODEL-REDUCER-001
**Title:** Decompose oversized reducer transition modules
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-PROJECTIONS-001 is DONE. Split reducer.rs and reducer
modules above approximately 500 lines into meaningful transition families.
Preserve dispatch order, state authority, public APIs and failure behavior.

```bash
cargo test -p yoctui-model --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
