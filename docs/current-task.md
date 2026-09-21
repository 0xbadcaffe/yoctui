# Current Task

**ID:** REDUCE-UI-CORE-001
**Title:** Decompose UI shell and primitive renderers
**Status:** NOT_STARTED

Dependency REDUCE-UI-WORKFLOWS-001 is DONE. Split `render.rs`, `primitives.rs`,
`dashboard_render.rs`, `footer.rs` and `telemetry_strip.rs` into meaningful
responsibility files of approximately 500 lines or less. Preserve responsive
layouts, focus, accessibility behavior and public APIs.

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
