# Current Task

## Task

**ID:** CLIENT-RUNTIME-MAINTENANCE-RELEASE-RUNNER-001
**Title:** Run release and utility maintenance jobs in the daemon
**Status:** IN_PROGRESS

## Objective

Route typed release, signature cache, build-history, and archive runners
through daemon ownership.

## Verification

```bash
cargo test -p yoctui client_runtime_maintenance_release
```
