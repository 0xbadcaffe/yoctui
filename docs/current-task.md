# Current Task

## Task

**ID:** CLIENT-RUNTIME-MAINTENANCE-SSTATE-001
**Title:** Run sstate maintenance jobs in the daemon
**Status:** IN_PROGRESS

## Objective

Route confirmed oe-check-sstate and sstate cleanup operations through
daemon-owned runners.

## Verification

```bash
cargo test -p yoctui client_runtime_maintenance_sstate
```
