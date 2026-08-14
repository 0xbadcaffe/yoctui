# Current Task

## Task

**ID:** LAYOUT-MODEL-001
**Title:** Add typed client-local pane layout tree
**Status:** IN_PROGRESS

## Objective

Add stable panes and horizontal/vertical splits, minimum dimensions, focus,
resize, serialization, and safe narrow collapse separately from daemon state.

## Verification

```bash
cargo test -p yoctui-model pane_layout
```
