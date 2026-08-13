# Current Task

## Task

**ID:** CLIENT-RUNTIME-QA-TASK-WORKER-001
**Title:** Move QA capability and task jobs into the daemon
**Status:** IN_PROGRESS

## Objective

Daemon owns typed QA task capability inspection and managed task lifecycle.

## Verification

```bash
cargo test -p yoctui client_runtime_qa_task
```
