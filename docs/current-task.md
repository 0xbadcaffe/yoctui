# Current Task

**ID:** REDUCE-UI-WORKFLOWS-001
**Title:** Decompose UI workflow renderers
**Status:** NOT_STARTED

Dependency REDUCE-APP-001 is DONE. Split `sdk_render.rs`, `testing_render.rs`
and `package_render.rs` into meaningful responsibility files of approximately
500 lines or less. Preserve typed model rendering, responsive behavior and
public APIs.

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
