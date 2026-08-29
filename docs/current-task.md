# Current Task

## Task

**ID:** DEVWORK-TERMINAL-001
**Title:** Prompt for embedded or detached interactive sessions
**Status:** IN_PROGRESS

## Objective

Present one explicit, focus-trapped destination chooser before interactive
build shell, devshell, menuconfig, Devtool workspace-shell, or edit-recipe
sessions are spawned.

## Dependencies

- DEVWORK-GOV-001 — DONE
- UX-TERMINAL-UX-001 — DONE
- PTY-DEVTOOL-001 — DONE
- PTY-MENUCONFIG-001 — DONE

## Definition of done

- Opening the chooser spawns no process and cancellation remains zero-spawn.
- Embedded is the default and creates the existing daemon-owned PTY request.
- Detached uses a detected, allowlisted terminal emulator without shell
  interpolation and is visibly unavailable when no graphical emulator exists.
- Build shell, devshell, menuconfig, Devtool workspace shell, and edit-recipe
  use the same destination policy.
- Focused model/app/UI/CLI tests pass.

## Verification

```bash
cargo test -p yoctui-model devwork_terminal
cargo test -p yoctui-app devwork_terminal
cargo test -p yoctui-ui devwork_terminal
cargo test -p yoctui -- devwork_terminal
```
