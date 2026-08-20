# Current Task

## Task

**ID:** MOUSE-UI-001
**Title:** Ensure mouse parity
**Status:** IN_PROGRESS

## Objective

Ensure mouse input reaches the same typed pane focus, selection, scrolling,
dialog, tab, terminal-session, and supported split-resize operations as the
keyboard without making any workflow mouse-dependent.

## Dependencies

- `NAV-UI-001` — DONE
- `TASKS-UI-003` — DONE
- `LOG-UI-002` — DONE
- `DIALOG-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-cli/tests/mouse_runtime.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Clicking Navigator, Workspace, or Inspector gives that pane the same typed
  focus and visible selection semantics as keyboard focus cycling.
- Clicking selectable rows and applicable tabs selects or activates the exact
  typed item without coordinate guesses outside the current responsive layout.
- Wheel input scrolls the pane under the pointer with bounded typed movement;
  modal dialogs trap mouse input just as they trap keyboard input.
- Dialog controls and choices support click activation only where the exact
  current geometry and typed action are authoritative.
- Existing split resizing and terminal-session selection retain mouse parity;
  unsupported resize boundaries remain inert and bounded.
- Every mouse operation has a documented keyboard route, and keyboard-only,
  no-color, reduced-motion, wide, narrow, and minimum modes remain complete.
- Input outside an actionable region causes no state mutation and no panic.

## Verification

```bash
cargo test -p yoctui-app next_generation_mouse
cargo test -p yoctui --test mouse_runtime next_generation_mouse
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
