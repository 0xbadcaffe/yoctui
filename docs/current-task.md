# Current Task

## Task

**ID:** CLIENT-RUNTIME-MAINTENANCE-SERVICE-RELEASE-001
**Title:** Move maintenance service and release jobs into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed PR service, signature, build-history, archive, and release
operations through daemon-owned runners.

## Verification

```bash
cargo test -p yoctui client_runtime_maintenance_service
```
