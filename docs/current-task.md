# Current Task

## Task

**ID:** TASKS-UI-001
**Title:** Redesign the Tasks table
**Status:** IN_PROGRESS

## Objective

Refine the Tasks table into a dense build monitor with adaptive authoritative
columns, strong running-row treatment, honest determinate and indeterminate
progress, and stable reduced-motion behavior.

## Dependencies

- `FOUNDATION-UI-003` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Columns hide deterministically according to the documented width priorities.
- Task, recipe, state, elapsed, real progress, and worker appear only when the
  typed model provides them.
- No task CPU value or ETA is fabricated.
- Running rows remain prominent without masking the selected row.
- Unknown progress uses a stable reduced-motion presentation and never appears
  determinate.

## Verification

```bash
cargo test -p yoctui-ui next_generation_tasks_table
cargo test -p yoctui-model task_rows
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
