# Current Task

## Task

**ID:** MULTICLIENT-PTY-001
**Title:** Define explicit multi-client PTY input ownership
**Status:** IN_PROGRESS

## Objective

Ensure many clients can view a daemon-owned PTY while exactly one explicit
writer lease controls input and resize, with stale epochs and disconnects
handled safely.

## Verification

```bash
cargo test -p yoctui-model pty_session
cargo test -p yoctui --test daemon_state_runtime daemon_runtime_accepts_a_second_client_while_the_first_is_idle
```
