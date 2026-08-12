# Current Task

## Task

**ID:** PTY-MODEL-001
**Title:** Add typed PTY session model
**Status:** IN_PROGRESS

## Objective

Add pure typed PTY session state with stable session ID and name, typed kind and
command identity, validated cwd/workspace ownership, lifecycle and exit state,
bounded dimensions and scrollback metadata, live process-group identity,
attached viewers and one writer lease, restartability, and checked reducer
transitions.

## Verification

```bash
cargo test -p yoctui-model pty_session
```
