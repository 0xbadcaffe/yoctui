# Current Task

## Task

**ID:** DAEMON-UI-001
**Title:** Add daemon and session status UI
**Status:** IN_PROGRESS

## Objective

Render connection/instance/BitBake/session/client/recovery state and confirmed
restart/stop actions.

## Verification

```bash
cargo test -p yoctui-ui daemon_status
```
