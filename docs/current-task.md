# Current Task

## Task

**ID:** CLIENT-RUNTIME-QA-WORKER-001
**Title:** Move QA report workers into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed QA capability, task and report jobs through daemon-owned bounded
workers.

## Verification

```bash
cargo test -p yoctui client_runtime_qa
```
