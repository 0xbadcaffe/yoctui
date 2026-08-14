# Current Task

## Task

**ID:** PANE-SPLIT-001
**Title:** Add explicit split-pane terminal sessions
**Status:** IN_PROGRESS

## Objective

Support horizontal/vertical split, close/focus/resize, existing-session
attachment, and new shell panes while retaining workbench primacy.

## Verification

```bash
cargo test -p yoctui-ui pane_split
```
