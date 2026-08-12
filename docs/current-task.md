# Current Task

## Task

**ID:** CLIENT-RUNTIME-JOBS-001
**Title:** Move remaining interactive job families behind daemon actions
**Status:** IN_PROGRESS

## Objective

Reuse the existing typed effects and job infrastructure to move Devtool, SDK,
QEMU, Wic, testing, QA, security, maintenance, and utility runner ownership out
of the interactive Ratatui process. Route requests through correlated daemon
commands/events and ensure client detach or termination never cancels
daemon-owned work.

## Verification

```bash
cargo test -p yoctui client_runtime_jobs
```
