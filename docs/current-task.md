# Current Task

## Task

**ID:** UX-KEYMAP-E2E-001
**Title:** Verify real-terminal menus keybindings focus and scrolling
**Status:** NOT_STARTED

## Objective

Verify the complete typed action catalog, effective keymap, menus, focus, and
scrolling through real terminal input rather than model-only dispatch.

## Dependencies

- `UX-PREFERENCES-001` — DONE
- `UX-FOCUS-001` — DONE
- `UX-SCROLL-001` — DONE

## Definition of done

- Every catalog default and custom chord dispatches to its typed route; invalid,
  colliding, reserved-prefix, unreachable, and disabled bindings fail closed.
- F10 application menus, context menus, palette routes, and popup modes retain
  focus ownership and exact disabled/safety explanations in a real PTY.
- Tab/Shift+Tab, pane subfocus, zoom/restore, row/page/edge/search scrolling,
  and mouse parity preserve the same selection and viewport authority.
- Keymap reset, persistence/restart, literal prefix handling, and narrow/wide
  transitions are covered without leaking input into dialogs or PTY writers.

## Verification

```bash
./scripts/test-tui-keymap.sh
./scripts/test-workbench-ux-keymap.sh
./scripts/test-tui-pty.sh
```
