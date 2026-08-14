# Current Task

## Task

**ID:** CLIENT-ARCH-001
**Title:** Refactor Ratatui UI into attachable daemon client
**Status:** IN_PROGRESS

## Objective

Client connects to daemon, requests/renders snapshots, sends typed actions,
subscribes to events, and detaches without owning long-running execution.

## Verification

```bash
cargo test -p yoctui client_arch
cargo test -p yoctui-ui client_arch
```
