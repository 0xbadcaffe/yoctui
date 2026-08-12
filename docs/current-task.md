# Current Task

## Task

**ID:** CLIENT-ARCH-001
**Title:** Refactor Ratatui UI into an attachable daemon client
**Status:** IN_PROGRESS

## Objective

Connect the interactive Ratatui client to the local daemon, request and install
authoritative snapshots, subscribe to ordered events, send typed actions, and
detach cleanly. Move any remaining long-running process ownership out of the
interactive client while preserving client-local presentation state.

## Verification

```bash
cargo test -p yoctui client_arch
cargo test -p yoctui-ui client_arch
```
