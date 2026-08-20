# Current Task

## Task

**ID:** RESPONSIVE-UI-001
**Title:** Add explicit breakpoint tests
**Status:** IN_PROGRESS

## Objective

Add a canonical breakpoint matrix covering 200x60, 160x50, 130x40, 100x30,
80x24, and below minimum. Prove that every layout is panic-free, preserves pane
priority and useful content, and never overlaps or clips dialog controls.

## Dependencies

- `HEADER-UI-001` — DONE
- `METRICS-UI-006` — DONE
- `DIALOG-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/tests/`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The exact six-size matrix renders representative Dashboard, Tasks, Logs,
  Recipes, Layers, and typed-dialog states without panic.
- 200x60, 160x50, and 130x40 preserve wide pane priority; 100x30 preserves the
  medium Inspector replacement contract; 80x24 preserves the focused narrow
  pane and switcher; below minimum renders only the resize instruction.
- No region overlap, orphaned border, or clipped dialog control is present in
  semantic TestBackend buffers.
- Useful state text, active focus, current selection, and contextual controls
  remain visible at each supported size.
- Resize transitions preserve selected pane and model selection identity.

## Verification

```bash
cargo test -p yoctui-ui breakpoint_matrix
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
