# Current Task

## Task

**ID:** CLIENT-RUNTIME-TESTING-001
**Title:** Move testing jobs into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed testing/selftest/import/export jobs through daemon ownership and
correlated lifecycle events.

## Verification

```bash
cargo test -p yoctui client_runtime_testing
```
