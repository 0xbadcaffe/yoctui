# Current Task

## Task

**ID:** DAEMON-CLI-001
**Title:** Add typed daemon and session CLI commands
**Status:** IN_PROGRESS

## Objective

Add typed daemon and session management commands for start, stop, restart,
status, attach, sessions, session attach, and session kill.

## Verification

```bash
cargo test -p yoctui daemon_cli
```
