# Current Task

## Task

**ID:** LAYOUT-RESTORE-001
**Title:** Restore client-local layout on reconnect
**Status:** IN_PROGRESS

## Objective

Restore layout separately from daemon global state and handle unavailable
sessions safely.

## Verification

```bash
cargo test -p yoctui layout_restore
```
