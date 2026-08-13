# Current Task

## Task

**ID:** CLIENT-RUNTIME-QA-REPORT-001
**Title:** Move QA jobs into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed QA capability, task and report jobs through daemon ownership.

## Verification

```bash
cargo test -p yoctui client_runtime_qa
```
