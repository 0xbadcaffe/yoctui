# Current Task

## Task

**ID:** RESOURCE-LIMITS-001
**Title:** Add daemon resource limits
**Status:** IN_PROGRESS

## Objective

Bound client/PTY counts, scrollback/log/history, IPC queues/requests/snapshots,
dimensions, and utility output.

## Verification

```bash
cargo test -p yoctui resource_limits
```
