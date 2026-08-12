# Current Task

## Task

**ID:** CLIENT-RUNTIME-ARTIFACTS-001
**Title:** Move SDK QEMU and Wic job ownership into the daemon
**Status:** IN_PROGRESS

## Objective

Extend the bounded protocol with closed typed SDK, QEMU, and Wic operations.
Route their existing effects through correlated daemon requests and sequenced
events, reuse current adapters/runners under daemon ownership, preserve normal
confirmation and cancellation rules, and ensure client detach never terminates
active artifact or emulator work.

## Verification

```bash
cargo test -p yoctui client_runtime_artifacts
```
