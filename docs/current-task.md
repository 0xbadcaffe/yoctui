# Current Task

## Task

**ID:** MULTICLIENT-001
**Title:** Support multiple daemon clients safely
**Status:** IN_PROGRESS

## Objective

Two terminals may attach, both receive global updates, focus/layout remains
client-local, and conflicting global actions serialize or reject clearly.

## Verification

```bash
cargo test -p yoctui multiclient
```
