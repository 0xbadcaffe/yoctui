# Current Task

## Task

**ID:** DAEMON-STATE-MODEL-001
**Title:** Partition daemon-global and client-local state
**Status:** IN_PROGRESS

## Objective

Define the pure typed daemon-global state and client replica/presentation
boundaries, including generation/sequence metadata, BitBake, project-profile,
workspace/session ownership, and bounded collection policy. Job-family
migration and runtime installation remain separate dependent tasks.

## Verification

```bash
cargo test -p yoctui-model daemon_state_partition
```
