# Current Task

## Task

**ID:** INPUT-TEST-003
**Title:** Test mouse interaction
**Status:** IN_PROGRESS

## Objective

Consolidate next-generation mouse acceptance across geometry-owned pane focus,
list/row selection, wheel scrolling, modal choices, and supported terminal
split resizing while preserving complete keyboard parity.

## Dependencies

- `MOUSE-UI-001` — DONE

## Relevant files

- `crates/yoctui-app/`
- `crates/yoctui-cli/tests/mouse_runtime.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- App-level tests cover wide/medium/narrow pane focus, exact Navigator/task/tab
  selection, workspace and dialog wheel routes, and inert unsupported regions.
- Runtime tests dispatch click focus and modal choices through the real client
  input composition path.
- PTY leaf selection and supported split drag resizing preserve exact pane ID,
  axis, ratio, and bounded typed actions.
- Every mouse route has a keyboard equivalent and modal input never leaks.

## Verification

```bash
cargo test -p yoctui-app next_generation_mouse
cargo test -p yoctui --test mouse_runtime next_generation_mouse
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
