# Current Task

## Task

**ID:** UI-VISION-TASKS-001
**Title:** Implement the task log history cockpit
**Status:** IN_PROGRESS

## Objective

Compose the Tasks workspace from a dense live task table, selected-task log
viewer, retained job history, and a structured task Inspector with real actions
and daemon/BitBake/system status.

## Dependencies

- `UI-VISION-NAV-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Wide Tasks shows the live table, selected-task typed log tail, and job history.
- The Inspector is divided into metadata, recent log, actions, and system status.
- Missing fields remain explicitly unavailable and no illustrative values appear.
- Reduced-height rendering preserves the primary task table without panicking.

## Verification

```bash
cargo test -p yoctui-ui workbench_tasks
cargo test -p yoctui-ui task_progress
cargo test -p yoctui-model background_job
./scripts/verify-roadmap.sh
```
