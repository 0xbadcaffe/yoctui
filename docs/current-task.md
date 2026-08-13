# Current Task

## Task

**ID:** CLIENT-RUNTIME-WIC-CREATE-001
**Title:** Move Wic image creation into the daemon
**Status:** IN_PROGRESS

## Objective

Route typed Wic image-creation effects through correlated daemon requests and
sequenced events, reusing the validated adapter/runner.

## Verification

```bash
cargo test -p yoctui client_runtime_wic_create
```
