# Current Task

## Task

**ID:** CLIENT-RUNTIME-MAINTENANCE-001
**Title:** Move maintenance jobs into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed maintenance, sstate, release, and utility jobs through daemon
ownership.

## Verification

```bash
cargo test -p yoctui client_runtime_maintenance
```
