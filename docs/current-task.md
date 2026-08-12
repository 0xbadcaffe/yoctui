# Current Task

## Task

**ID:** BITBAKE-SERVER-001
**Title:** Create daemon-owned BitBake controller
**Status:** IN_PROGRESS

## Objective

Create a daemon-owned, UI-independent BitBake controller abstraction with typed
detect, connect, disconnect, start, stop, restart, and reconnect lifecycle.
Prefer supported BitBake interfaces, preserve structured capability/version
state, enforce timeouts, and make failure/recovery transitions explicit.

## Verification

```bash
cargo test -p yoctui-bitbake server_controller
```
