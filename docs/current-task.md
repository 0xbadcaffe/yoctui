# Current Task

## Task

**ID:** CLIENT-RUNTIME-TEST-JUNIT-WORKER-001
**Title:** Move JUnit export into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed JUnit export through daemon-owned validated resulttool runner and
lifecycle events.

## Verification

```bash
cargo test -p yoctui client_runtime_test_junit
```
