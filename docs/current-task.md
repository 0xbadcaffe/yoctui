# Current Task

## Task

**ID:** PANE-SPLIT-RUNTIME-001
**Title:** Wire split-pane actions and session attachment
**Status:** IN_PROGRESS

## Objective

Typed actions must split/close/focus/resize panes, attach existing PTYs, create
shells, and preserve workbench navigation without notification-only
placeholders.

## Verification

```bash
cargo test -p yoctui --test pane_split_runtime pane_split
```
