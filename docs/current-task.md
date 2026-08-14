# Current Task

## Task

**ID:** CLIENT-RUNTIME-QA-TASK-RUNNER-001
**Title:** Move QA task capability and checks into the daemon
**Status:** IN_PROGRESS

## Objective

Daemon routes QA capability and task checks through typed daemon commands so
the interactive client does not own long-running QA execution.

## Verification

```bash
cargo test -p yoctui client_runtime_qa_task
```
