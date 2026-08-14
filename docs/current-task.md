# Current Task

## Task

**ID:** DAEMON-REBOOT-TEST-001
**Title:** Test daemon restart and recovery acceptance
**Status:** IN_PROGRESS

## Objective

Simulate daemon death/restart and verify metadata reload, honest Lost children,
BitBake/client reconnect, and session recovery semantics.

## Verification

```bash
cargo test -p yoctui daemon_reboot
```
