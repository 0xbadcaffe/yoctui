# Current Task

## Task

**ID:** DAEMON-SNAPSHOT-001
**Title:** Synchronize snapshots and incremental daemon events
**Status:** IN_PROGRESS

## Objective

Provide a bounded consistent state snapshot followed by ordered incremental
events without a subscription gap. Track daemon sequence/generation, replay
missed events on reconnect where retained, require safe replacement when a
client is stale, and keep snapshot/replay queues within configured limits.

## Verification

```bash
cargo test -p yoctui-protocol daemon_snapshot
cargo test -p yoctui-app daemon_snapshot
```
