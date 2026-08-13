# Current Task

## Task

**ID:** CLIENT-RUNTIME-TEST-RESULTS-001
**Title:** Move test-result import and export jobs into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed result import, comparison and JUnit export jobs through daemon
ownership and correlated lifecycle events.

## Verification

```bash
cargo test -p yoctui client_runtime_test_results
```
