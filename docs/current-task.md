# Current Task

## Task

**ID:** REMOTE-SSH-001
**Title:** Validate local daemon reattachment over SSH
**Status:** IN_PROGRESS

## Objective

Client attaches to build-host local socket after SSH login; SSH loss cannot
stop daemon work and no unauthenticated TCP daemon is added.

## Verification

```bash
cargo test -p yoctui ssh_reattach
```
