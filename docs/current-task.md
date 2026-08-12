# Current Task

## Task

**ID:** PTY-ATTACH-001
**Title:** Implement PTY attach and detach
**Status:** IN_PROGRESS

## Objective

Integrate daemon-owned runner and emulator sessions with typed client
attach/detach. A prefix return or detach must leave the PTY alive, client exit
must release only its viewer/writer lease, attach and reattach must receive the
current bounded terminal snapshot, and session listings must distinguish
Running, Exited and Lost honestly.

## Verification

```bash
cargo test -p yoctui pty_attach
```
