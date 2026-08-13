# Current Task

## Task

**ID:** CLIENT-RUNTIME-WIC-001
**Title:** Move Wic job ownership into the daemon
**Status:** IN_PROGRESS

## Objective

Extend the bounded protocol with closed typed Wic operations. Route confirmed
Wic effects through correlated daemon requests and sequenced events, reusing
the validated adapter/runner while preserving destructive device confirmation.

## Verification

```bash
cargo test -p yoctui client_runtime_wic
```
