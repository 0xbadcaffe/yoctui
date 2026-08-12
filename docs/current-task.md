# Current Task

## Task

**ID:** PTY-DEVTOOL-001
**Title:** Route interactive Devtool through daemon PTYs
**Status:** IN_PROGRESS

## Objective

Route interactive Devtool workflows through typed daemon-owned PTY requests.
Reuse authoritative recipe/Devtool workspace identity and captured build
environment, construct only allowlisted shell-free Devtool argv, preview the
exact operation, reject stale context, and preserve noninteractive Devtool job
paths for workflows that do not require a terminal.

## Verification

```bash
cargo test -p yoctui-app pty_devtool
```
