# Current Task

## Task

**ID:** CLIENT-RUNTIME-TEST-RESULTTOOL-001
**Title:** Move resulttool capability ownership into the daemon
**Status:** IN_PROGRESS

## Objective

Daemon owns validated resulttool discovery and exposes typed capability state
for JUnit and comparison operations.

## Verification

```bash
cargo test -p yoctui daemon_test_resulttool
```
