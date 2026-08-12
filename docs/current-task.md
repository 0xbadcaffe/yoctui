# Current Task

## Task

**ID:** DAEMON-STATE-RUNTIME-001
**Title:** Make daemon runtime own authoritative state
**Status:** IN_PROGRESS

## Objective

Install the typed global state in the foreground daemon runtime, route typed
mutations through its reducer, and expose client replicas without moving
long-running ownership back into the interactive client.

## Verification

```bash
cargo test -p yoctui daemon_state_runtime
cargo test -p yoctui-app daemon_state_runtime
```
