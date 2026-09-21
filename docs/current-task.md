# Current Task

**ID:** REDUCE-APP-INPUT-001
**Title:** Decompose application input routing
**Status:** NOT_STARTED

Dependency REDUCE-BITBAKE-001 is DONE. Audit `crates/yoctui-app`, split sources
`mouse.rs`, `workspace_input.rs`, `dialog_input.rs` and `keyboard.rs` into
meaningful responsibility files of approximately 500 lines or less. Preserve
typed actions, focus behavior and public APIs.

```bash
cargo test -p yoctui-app --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
