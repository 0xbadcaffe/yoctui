# Current Task

## Task

**ID:** CLIENT-RUNTIME-SECURITY-MAPPER-001
**Title:** Run security package mapping in the daemon
**Status:** IN_PROGRESS

## Objective

Route confirmed cve-check-map-pkgs operations through daemon-owned typed
process execution.

## Verification

```bash
cargo test -p yoctui client_runtime_security_mapper
```
