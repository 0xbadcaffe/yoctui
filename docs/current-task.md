# Current Task

## Task

**ID:** CLIENT-RUNTIME-MAINTENANCE-RELEASE-001
**Title:** Move release and utility maintenance jobs into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed signature cache, build-history, archive, and release operations
through daemon-owned runners.

## Verification

```bash
cargo test -p yoctui client_runtime_maintenance_release
```
