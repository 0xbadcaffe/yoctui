# Current Task

## Task

**ID:** DAEMON-LIFECYCLE-001
**Title:** Implement daemon lifecycle commands
**Status:** IN_PROGRESS

## Objective

Implement `yoctui daemon start/status/stop/restart`, a foreground debug mode,
typed automatic client connection with optional auto-start, secure PID/runtime
state, crash recovery, and clean socket cleanup using a robust Rust service
model rather than shell-specific daemonization.

## Verification

```bash
cargo test -p yoctui daemon_lifecycle
cargo test -p yoctui-protocol daemon_lifecycle
```
