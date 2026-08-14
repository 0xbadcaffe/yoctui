# Current Task

## Task

**ID:** SECURITY-DAEMON-001
**Title:** Harden daemon IPC and process ownership
**Status:** IN_PROGRESS

## Objective

Enforce socket/peer/path safety, command and PTY ownership validation,
environment filtering, profile trust rules, and no escalation.

## Verification

```bash
cargo test -p yoctui security_daemon
cargo test -p yoctui-protocol security_daemon
```
