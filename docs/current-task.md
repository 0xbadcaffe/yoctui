# Current Task

## Task

**ID:** CLIENT-RUNTIME-QA-CHECK-RUNNER-001
**Title:** Run managed QA checks in the daemon
**Status:** IN_PROGRESS

## Objective

Daemon owns managed QA check process runners, output, cancellation and
terminal state.

## Verification

```bash
cargo test -p yoctui client_runtime_qa_task
```
