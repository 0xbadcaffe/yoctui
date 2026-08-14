# Current Task

## Task

**ID:** PTY-TEST-001
**Title:** Add real PTY integration tests
**Status:** IN_PROGRESS

## Objective

Use real PTYs to test prompt, typing, resize, detach/reattach, sessions, exit,
cancellation, ncurses fixture, raw keys, mouse where supported, and scrollback
bounds.

## Verification

```bash
cargo test -p yoctui pty_integration
```
