# Current Task

## Task

**ID:** LOG-UI-001
**Title:** Redesign live Log Viewer
**Status:** IN_PROGRESS

## Objective

Make live logs easy to follow without moving parsing into rendering: show exact
task/recipe context, bounded position, normalized search emphasis, and only
actions backed by authoritative log paths.

## Dependencies

- `FOUNDATION-UI-003` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The section header reports selected recipe/task context and follow/pause.
- Vertical and horizontal position indicators are bounded and truthful.
- Warning/error lines and normalized search hits use semantic emphasis.
- Retained ANSI-safe normalized text remains the only rendered log source.
- Full-log and source-log actions appear only when their typed path exists.
- Empty, loading, error, and eviction states remain explicit and bounded.

## Verification

```bash
cargo test -p yoctui-ui next_generation_log_viewer
cargo test -p yoctui-model log_state
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
