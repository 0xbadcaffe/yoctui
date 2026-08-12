# Current Task

## Task

**ID:** CLIENT-REPLICA-001
**Title:** Install daemon state into the interactive client replica
**Status:** IN_PROGRESS

## Objective

Map a bounded authoritative daemon snapshot and ordered incremental events into
typed state consumed by the Ratatui client. Preserve focus, screen, theme,
dialogs, editor state, layout, and other presentation choices locally while
daemon-owned BitBake, jobs, logs, recovery, clients, and PTY summaries replace
stale replicas safely.

## Verification

```bash
cargo test -p yoctui-app client_replica
cargo test -p yoctui-ui client_replica
```
