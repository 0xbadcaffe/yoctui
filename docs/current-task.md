# Current Task

## Task

**ID:** DAEMON-TELEMETRY-001
**Title:** Expose daemon and session health telemetry
**Status:** IN_PROGRESS

## Objective

Report uptime, BitBake state, clients, jobs, PTYs, queue pressure, available
memory data, and restart/recovery state.

## Verification

```bash
cargo test -p yoctui-model daemon_telemetry
```
