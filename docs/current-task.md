# Current Task

## Task

**ID:** CLIENT-RUNTIME-TEST-SESSION-001
**Title:** Move test and selftest sessions into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed selftest and managed test-build sessions through daemon ownership
and correlated lifecycle events.

## Verification

```bash
cargo test -p yoctui client_runtime_test_session
```
