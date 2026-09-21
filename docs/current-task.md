# Current Task

**ID:** REDUCE-UI-WORKSPACES-001
**Title:** Decompose remaining UI workspace renderers
**Status:** NOT_STARTED

Dependency REDUCE-UI-CORE-001 is DONE. Split `security_render.rs`,
`qa_render.rs`, `maintenance_render.rs`, `inspector_workspace.rs`,
`inspector_render.rs`, `task_render.rs`, `terminal_workspace.rs`,
`rootfs_render.rs` and `source_render.rs` into meaningful responsibility files
of approximately 500 lines or less. Complete the production-source audit while
preserving typed state, narrow-terminal safety and public APIs.

```bash
cargo test -p yoctui-ui --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
