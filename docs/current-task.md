# Current Task

## Task

**ID:** DAEMON-STATE-001
**Title:** Move authoritative long-lived state into daemon
**Status:** IN_PROGRESS

## Objective

Move BitBake connection state, all background-job families, bounded logs and
errors, task history, QEMU/Wic/SDK/testing/QA/security/maintenance/utility
state, PTY sessions, project-profile state, and session metadata into one typed
daemon-owned authoritative model. Clients must render this state rather than
owning long-running execution.

## Verification

```bash
cargo test -p yoctui-model daemon_state
cargo test -p yoctui-app daemon_state
```
