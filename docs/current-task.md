# Current Task

## Task

**ID:** DAEMON-RECOVERY-001
**Title:** Recover honestly after daemon restart
**Status:** IN_PROGRESS

## Objective

Reload validated persisted metadata after daemon restart. Restore client-visible
history and session names, classify formerly live jobs and unrecoverable PTYs
as `Lost`, identify BitBake reconnection intent without claiming a connection,
and restore only external services whose supported interface proves them live.

## Verification

```bash
cargo test -p yoctui daemon_recovery
```
