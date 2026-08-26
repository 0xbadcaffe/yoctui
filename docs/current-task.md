# Current Task

## Task

**ID:** UX-TERMINAL-UX-001
**Title:** Complete the first-class built-in terminal workbench
**Status:** NOT_STARTED

## Objective

Turn daemon-owned terminal sessions into a complete discoverable workbench
without weakening writer leases, typed input routing, replica bounds, or honest
process lifecycle reporting.

## Dependencies

- `UX-TERMINAL-EVAL-001` — DONE
- `UX-FOCUS-001` — DONE
- `UX-SCROLL-001` — DONE
- `UX-KEYMAP-MODEL-001` — DONE

## Relevant files

- Terminal Sessions navigation destination and context-aware creation
- session list/tabs, splits, zoom, rename, detach, close, and confirmed kill
- explicit viewer, writer, read-only, and take-control state
- copy mode, search, paste, scrollback, and dropped-history accounting
- shell, devshell, menuconfig, SDK, Devtool, and Raw session identities
- prefix Help, literal-prefix forwarding, reconnect, exit, and loss outcomes

## Definition of done

- Terminal Sessions is reachable from navigation, menus, palette, and relevant
  workspace actions with the same typed availability.
- Every session operation preserves single-writer/multi-viewer ownership and
  requires explicit confirmation before process-group termination.
- Copy/search/paste/scrollback are bounded, keyboard discoverable, and cannot
  leak input outside the terminal owner or bypass the configured prefix.
- Responsive, no-color, ASCII, reconnect, exited, lost, dropped-history,
  TestBackend, CLI routing, and real-PTY states are covered.

## Verification

```bash
cargo test -p yoctui-model ux_terminal
cargo test -p yoctui-app ux_terminal
cargo test -p yoctui-ui ux_terminal
cargo test -p yoctui -- ux_terminal
```
