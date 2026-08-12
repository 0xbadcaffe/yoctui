# Current Task

## Task

**ID:** BITBAKE-RESTART-001
**Title:** Implement controlled BitBake restart
**Status:** IN_PROGRESS

## Objective

Implement a controlled daemon-owned BitBake restart workflow. Preview affected
active jobs, reject unsafe restarts unless the exact operation is explicitly
confirmed, disconnect and stop through supported adapters, restart and
reconnect, refresh authoritative metadata, and preserve daemon/client state
where safe.

## Verification

```bash
cargo test -p yoctui-bitbake bitbake_restart
cargo test -p yoctui-model bitbake_restart
```
