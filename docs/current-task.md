# Current Task

## Task

**ID:** CLIENT-RUNTIME-TEST-SNAPSHOT-001
**Title:** Define typed daemon test-result snapshots
**Status:** IN_PROGRESS

## Objective

Add bounded protocol types for test-result records, limitations, import
generations and comparison state so result workers can migrate without leaking
model internals into UI code.

## Verification

```bash
cargo test -p yoctui daemon_test_snapshot
```
