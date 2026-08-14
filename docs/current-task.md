# Current Task

## Task

**ID:** CLIENT-RUNTIME-JOBS-001
**Title:** Move remaining interactive job families behind daemon actions
**Status:** IN_PROGRESS

## Objective

All long-lived job families reuse typed daemon actions/events; interactive
client shutdown owns no runner cleanup and detach leaves daemon work running.

## Verification

```bash
cargo test -p yoctui client_runtime_jobs
```
