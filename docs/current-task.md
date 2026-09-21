# Current Task

**ID:** REDUCE-MODEL-RAW-CATALOG-001
**Title:** Decompose the built-in raw command catalog
**Status:** NOT_STARTED

Dependency REDUCE-UTILS-001 is DONE. Split the 11,852-line built-in raw command
catalog into meaningful command-family modules targeting approximately 500
lines each. Preserve deterministic catalog order, command/category/executable
counts, the reviewed reference hash and every public behavior.

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
