# Current Task

## Task

**ID:** PTY-MULTI-001
**Title:** Support bounded multiple PTY sessions
**Status:** IN_PROGRESS

## Objective

Add a bounded daemon PTY session registry with stable non-reused IDs, validated
unique names, create/close/rename/switch and history operations, explicit
resource-limit failures, and no single-active-session assumption. Closing one
session must affect only its owned runner/process group.

## Verification

```bash
cargo test -p yoctui-model pty_multi
```
