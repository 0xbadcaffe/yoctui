# Current Task

## Task

**ID:** MULTICLIENT-RUNTIME-001
**Title:** Support simultaneous daemon client connections
**Status:** IN_PROGRESS

## Objective

Run concurrent attached clients with global event fan-out and explicit
serialization/rejection of conflicting commands.

## Verification

```bash
cargo test -p yoctui multiclient
```
