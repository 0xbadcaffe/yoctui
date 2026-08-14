# Current Task

## Task

**ID:** SSH-TEST-001
**Title:** Test SSH-style disconnect and reconnect
**Status:** IN_PROGRESS

## Objective

Verify that client termination does not terminate daemon work using a local
pseudo-SSH/process fixture, with optional real SSH coverage.

## Verification

```bash
cargo test -p yoctui ssh_integration
```
