# Current Task

## Task

**ID:** DAEMON-PROTOCOL-001
**Title:** Define typed daemon client protocol
**Status:** IN_PROGRESS

## Objective

Add versioned bounded daemon/client wire types for handshake and capabilities,
identities, subscriptions, snapshots and incremental events, correlated
commands, jobs, PTYs, layout/mouse events, attach/detach, graceful shutdown,
errors, stale-client detection, and reconnect synchronization with explicit
compatibility rules.

## Verification

```bash
cargo test -p yoctui-protocol daemon_protocol
```
