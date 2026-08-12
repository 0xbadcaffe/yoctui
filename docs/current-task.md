# Current Task

## Task

**ID:** DAEMON-PERSIST-001
**Title:** Persist safe daemon metadata
**Status:** IN_PROGRESS

## Objective

Persist only safe, meaningful daemon metadata: workspace/profile identity, job
history, terminal session and layout metadata, session names, configured
bounded recent logs, user preferences, and reconnect metadata. Never serialize
or imply that live child PIDs survive daemon or host restart.

## Verification

```bash
cargo test -p yoctui daemon_persist
```
