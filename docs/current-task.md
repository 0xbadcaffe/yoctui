# Current Task

## Task

**ID:** CLIENT-RUNTIME-WIC-DEVICE-001
**Title:** Move Wic device writes into the daemon
**Status:** IN_PROGRESS

## Objective

Route confirmed destructive Wic device-write effects through correlated daemon
requests and sequenced events, retaining exact device identity safeguards.

## Verification

```bash
cargo test -p yoctui client_runtime_wic_device
```
