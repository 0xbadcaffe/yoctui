# Current Task

## Task

**ID:** UI-REGRESSION-001
**Title:** Verify all existing functionality remains available
**Status:** IN_PROGRESS

## Objective

Prove every pre-existing workspace, daemon/session function, and
capability-correlated route remains reachable after the UI changes.

## Dependencies

- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE
- `RAW-RESPONSIVE-001` — DONE
- `VISUAL-TEST-003` — DONE
- `INPUT-TEST-002` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Existing workspace and daemon/session routes remain reachable.
- Capability-correlated actions retain their typed gating and effects.
- Regression tests cover utility coverage and compatibility invariants.

## Verification

```bash
cargo test --workspace --all-features ui_regression
./scripts/verify-utility-coverage.sh
./scripts/verify-compatibility.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
