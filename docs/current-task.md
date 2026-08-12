# Current Task

## Task

**ID:** CLIENT-RUNTIME-001
**Title:** Wire Ratatui runtime to daemon-owned execution
**Status:** IN_PROGRESS

## Objective

Connect interactive startup and the terminal event loop to the typed daemon
transport and client replica. Route global execution actions to daemon commands,
process daemon state updates without blocking UI input, detach cleanly on exit,
and remove remaining interactive-client ownership of long-running work without
creating a parallel command path.

## Verification

```bash
cargo test -p yoctui client_runtime
cargo test -p yoctui-ui client_runtime
```
