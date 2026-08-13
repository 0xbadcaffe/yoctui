# Current Task

## Task

**ID:** CLIENT-RUNTIME-TEST-IMPORT-001
**Title:** Move test-result import and comparison into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed result import and comparison operations through daemon-owned
bounded workers and correlated lifecycle events. The existing result snapshot
protocol must be extended before client-local result state can migrate safely.

## Verification

```bash
cargo test -p yoctui client_runtime_test_import
```
