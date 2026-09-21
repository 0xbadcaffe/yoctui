# Current Task

**ID:** REDUCE-MODEL-INTERACTION-001
**Title:** Decompose text input keymap and action catalogs
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-WORKFLOWS-001 is DONE. Split textarea.rs, keymap.rs,
action_catalog.rs and related interaction types into meaningful editing,
navigation, key binding and catalog projection modules. Preserve public APIs
and typed interaction behavior while targeting approximately 500 lines per
source.

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
