# Current Task

## Task

**ID:** DAEMON-SERVICE-INTEGRATION-001
**Title:** Add systemd user service integration
**Status:** IN_PROGRESS

## Objective

Generate, install, and manage a systemd user service where available without
root. Support start, stop, restart, and status, document daemon auto-start, and
provide a clear direct-process fallback when the user service manager is
unavailable.

## Verification

```bash
cargo test -p yoctui daemon_service
./scripts/check-docs.sh
```
