# Current Task

## Task

**ID:** DAEMON-TEST-001
**Title:** Add daemon protocol and lifecycle integration tests
**Status:** IN_PROGRESS

## Objective

Cover handshake, reconnect, stale sockets, multiple/dropped clients, restart,
BitBake loss, ordering, snapshots, malformed protocol, and limits.

## Verification

```bash
cargo test -p yoctui daemon_integration
```
