# Current Task

## Task

**ID:** MOUSE-RUNTIME-001
**Title:** Route widget and terminal mouse interactions
**Status:** IN_PROGRESS

## Objective

Add row/tree clicks, dialog controls, scrollbar and drag semantics, terminal
focus/session tabs, and server-relevant PTY mouse reports with explicit writer
ownership.

## Verification

```bash
cargo test -p yoctui-ui mouse_runtime
cargo test -p yoctui --test mouse_runtime mouse
```
