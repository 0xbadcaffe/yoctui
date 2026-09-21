# Current Task

**ID:** REDUCE-MODEL-QA-001
**Title:** Decompose QA report and workflow models
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-MAINTENANCE-001 is DONE. Split the 3,707-line QA model
into meaningful modules for report identity, finding normalization, imports,
workflow state, filters/projections and transitions. Preserve typed requests,
public APIs, authority rules and reducer behavior while targeting approximately
500 lines per production source.

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
