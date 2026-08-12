# Current Task

## Task

**ID:** DAEMON-SPEC-001
**Title:** Specify daemon and attachable client architecture
**Status:** IN_PROGRESS

## Objective

Define the daemon/client responsibilities, long-lived ownership and persistence
boundaries, IPC and attach semantics, lifecycle and recovery behavior,
compatibility, multi-client security, SSH/reboot guarantees, and interaction
with the existing single-process mode before implementation begins.

## Verification

```bash
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
