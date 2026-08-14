# Current Task

## Task

**ID:** MULTICLIENT-PTY-RUNTIME-001
**Title:** Route multi-client PTY ownership through the daemon
**Status:** IN_PROGRESS

## Objective

Expose create/attach/detach/take-control/input/resize through daemon-owned
sessions, publish typed PTY state/output events to all viewers, and release
writer leases when a client disconnects.

## Verification

```bash
cargo test -p yoctui --test daemon_pty_runtime multiclient_pty
```
