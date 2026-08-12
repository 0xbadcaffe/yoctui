# Current Task

## Task

**ID:** CLIENT-TRANSPORT-001
**Title:** Implement attachable daemon client transport
**Status:** IN_PROGRESS

## Objective

Add one typed client session over the secure local IPC transport. It negotiates
the version and capabilities, attaches with an optional resume cursor, receives
bounded snapshots/events/results, reconnects with clear diagnostics, and
explicitly detaches without affecting daemon-owned work.

## Verification

```bash
cargo test -p yoctui client_transport
```
