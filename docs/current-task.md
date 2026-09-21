# Current Task

**ID:** REDUCE-MODEL-REMAINDER-001
**Title:** Decompose remaining oversized model sources
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-REDUCER-001 is DONE. Audit every remaining
`yoctui-model` production source above approximately 500 lines and split it into
meaningful modules. Preserve public exports and pure reducer boundaries.

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
