# Current Task

## Task

**ID:** DAEMON-IPC-001
**Title:** Implement secure local daemon IPC
**Status:** IN_PROGRESS

## Objective

Implement local-only Unix-domain transport with deterministic secure runtime
paths, socket permissions, peer verification where available, symlink-safe
stale cleanup, bounded messages, reconnect and timeouts, and actionable daemon
unavailable diagnostics. Do not open a network listener.

## Verification

```bash
cargo test -p yoctui-protocol daemon_ipc
```
