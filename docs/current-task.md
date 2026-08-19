# Current Task

## Task

**ID:** TASKS-UI-003
**Title:** Add high-quality task state visualization
**Status:** IN_PROGRESS

## Objective

Make every task lifecycle state distinct through stable text, a terminal-safe
marker, and semantic styling without relying on color alone.

## Dependencies

- `TASKS-UI-002` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Queued, waiting, active, succeeded, failed, cancelled, and lost meanings are
  visually and textually distinct.
- No state relies on color alone.
- Status labels remain stable in Unicode-disabled/no-color modes.
- Task and aggregate waiting rows preserve their exact distinct meaning.
- State presentation is covered across selected and unselected rows.

## Verification

```bash
cargo test -p yoctui-ui next_generation_task_states
cargo test -p yoctui-model task_state
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
