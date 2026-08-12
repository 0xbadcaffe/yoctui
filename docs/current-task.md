# Current Task

## Task

**ID:** BITBAKE-SOCKET-001
**Title:** Integrate supported BitBake server socket APIs
**Status:** IN_PROGRESS

## Objective

Implement the supported BitBake socket/server adapter beneath the typed
controller. Detect capabilities and versions, enforce connect/command timeouts,
correlate commands, handle server loss and reconnect explicitly, and keep all
transport details independent of UI state.

## Verification

```bash
cargo test -p yoctui-bitbake bitbake_socket
```
