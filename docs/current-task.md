# Current Task

## Task

**ID:** DAEMON-STATE-001
**Title:** Move authoritative long-lived state into daemon
**Status:** IN_PROGRESS

## Objective

Verify the parent state-ownership gate: the daemon owns typed BitBake state,
bounded logs/errors/history, all background workflow families, PTY metadata,
project-profile state, and session metadata while interactive clients consume
replaceable replicas and retain only client-local presentation.

## Verification

```bash
cargo test -p yoctui-model daemon_state
cargo test -p yoctui-app daemon_state
```
