# Current Task

## Task

**ID:** CLIENT-RUNTIME-001
**Title:** Wire Ratatui runtime to daemon-owned execution
**Status:** IN_PROGRESS

## Objective

Startup/event loop uses daemon state/actions, detach is clean, and no
long-running process ownership remains in the interactive client path.

## Verification

```bash
cargo test -p yoctui client_runtime_jobs
cargo test -p yoctui client_runtime
```
