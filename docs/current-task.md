# Current Task

## Task

**ID:** UI-REGRESSION-001
**Title:** Verify all existing functionality remains available
**Status:** IN_PROGRESS

## Objective

Prove every pre-existing workspace, daemon/session function, and
capability-correlated route remains reachable after the redesign.

## Dependencies

- `VISUAL-TEST-003` — DONE
- `INPUT-TEST-002` — DONE

## Relevant files

- `crates/yoctui-e2e/`
- `scripts/verify-utility-coverage.sh`
- `scripts/verify-compatibility.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

 - Every existing workspace and daemon/session route remains reachable.
 - Capability-correlated actions retain their typed routes and disabled reasons.
 - Regression tests and utility/compatibility verification pass without weakened checks.

## Verification

```bash
cargo test --workspace --all-features ui_regression
./scripts/verify-utility-coverage.sh
./scripts/verify-compatibility.sh
```
