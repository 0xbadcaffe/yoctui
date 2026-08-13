# Current Task

## Task

**ID:** CLIENT-RUNTIME-TEST-COMPARE-001
**Title:** Move test-result comparison into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed comparison requests through daemon-owned bounded workers and diff
events.

## Verification

```bash
cargo test -p yoctui client_runtime_test_compare
```
