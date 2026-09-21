# Current Task

**ID:** REDUCE-MODEL-COMPATIBILITY-001
**Title:** Decompose compatibility state catalogs and projections
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-SECURITY-TESTING-001 is DONE. Split compatibility.rs,
compatibility_catalog.rs, compatibility_ui.rs and workspace_compatibility.rs
into meaningful modules for capability identity, implementation selection,
catalogs, UI projections and workspace behavior. Preserve public APIs and
fail-closed authority while targeting approximately 500 lines per source.

```bash
cargo test -p yoctui-model --all-features raw
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
