# Current Task

## Task

**ID:** PTY-SPEC-001
**Title:** Specify daemon-owned PTY session architecture
**Status:** IN_PROGRESS

## Objective

Specify daemon ownership of PTYs, terminal emulation, process groups, validated
environment and working directory, stable session identity and dimensions,
bounded byte input/output and scrollback, resize, attach/detach and multi-client
semantics, termination, copy/search modes, and paste policy. Define the boundary
before runner or UI implementation.

## Verification

```bash
./scripts/check-docs.sh
```
