# Current Task

## Task

**ID:** CLIENT-RUNTIME-QA-ADAPTER-001
**Title:** Run QA capability inspection in the daemon
**Status:** IN_PROGRESS

## Objective

Daemon reconstructs bounded QA input and invokes QaTaskCapabilityInspector,
publishing typed capability snapshots.

## Verification

```bash
cargo test -p yoctui client_runtime_qa_adapter
```
