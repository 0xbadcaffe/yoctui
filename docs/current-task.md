# Current Task

## Task

**ID:** CRATESIO-BRIDGE-001
**Title:** Bundle the BitBake bridge for installed binaries
**Status:** IN_PROGRESS

## Objective

Make the default bridge backend self-contained in a compiled Yoctui binary so
`cargo install yoctui` does not depend on the source checkout. Preserve the
explicit `YOCTUI_BRIDGE_PATH` override for development and diagnostics.

## Dependencies

- `DAEMON-UPGRADE-LIFECYCLE-001` — DONE

## Relevant files

- `bridge/yoctui_bridge.py`
- `crates/yoctui-bitbake/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-cli/src/daemon_bitbake.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- The production bridge source is compiled into the Rust package boundary.
- Default standalone and daemon bridge startup work without a checkout path.
- `YOCTUI_BRIDGE_PATH` continues to select an explicit external bridge.
- Focused and baseline checks pass.
- Registry/status/current-task documentation is updated and committed.

## Verification

```bash
cargo test -p yoctui-bitbake bundled_bridge
cargo test -p yoctui bundled_bridge
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
