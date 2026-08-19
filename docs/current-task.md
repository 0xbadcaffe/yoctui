# Current Task

## Task

**ID:** INSPECTOR-UI-002
**Title:** Add task Inspector
**Status:** IN_PROGRESS

## Objective

Render the selected task as a dense typed Inspector document using only task,
recipe, dependency, path, and bounded log data already present in the model.

## Dependencies

- `INSPECTOR-UI-001` — DONE
- `TASKS-UI-001` — DONE
- `LOG-UI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Primary facts show task, recipe/PN, lifecycle, progress, start, elapsed,
  worker, and PID from typed task state.
- PV is shown only when the authoritative recipe inventory provides it; PR and
  workdir remain explicitly unavailable unless an authoritative field exists.
- The log path and typed task dependencies are visible in their own sections.
- Recent output is a bounded tail correlated by selected recipe/task identity.
- Waiting aggregate rows remain explicit and do not acquire invented metadata.
- Wide, overlay, and narrow Inspector layouts remain readable and safe.

## Verification

```bash
cargo test -p yoctui-ui next_generation_task_inspector
cargo test -p yoctui-model task_inspector
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
