# Current Task

## Task

**ID:** PTY-SDK-SHELL-001
**Title:** Support SDK and native environment PTYs
**Status:** IN_PROGRESS

## Objective

Safely capture a selected SDK or native toolchain environment without mutating
the Yoctui process, validate its identity and paths, and open a persistent
interactive shell through the daemon-owned PTY infrastructure. Ensure stale or
untrusted environment setup inputs fail closed and exact launch intent remains
previewable.

## Verification

```bash
cargo test -p yoctui pty_sdk_shell
```
