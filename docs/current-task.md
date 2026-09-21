# Current Task

**ID:** REDUCE-MODEL-PROJECTIONS-001
**Title:** Decompose dashboard progress and daemon projections
**Status:** NOT_STARTED

Dependency REDUCE-MODEL-INTERACTION-001 is DONE. Split overview.rs,
dashboard.rs, progress.rs, widget_projection.rs, daemon_state.rs, pty_session.rs
and session_updates.rs into meaningful pure state and projection modules.
Preserve public APIs and typed behavior while targeting approximately 500 lines
per source.

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
