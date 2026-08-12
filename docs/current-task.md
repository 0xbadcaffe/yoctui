# Current Task

## Task

**ID:** PTY-RUNNER-001
**Title:** Implement daemon-owned Unix PTY runner
**Status:** IN_PROGRESS

## Objective

Implement a daemon-owned Unix PTY runner using the typed session model. Own the
master and child process group, carry interactive raw bytes in both directions,
apply validated resize and terminal modes, bound output and queues, support
graceful then forced cancellation, reap children and descendants, and report
typed lifecycle/output/exit/loss events without UI dependencies.

## Verification

```bash
cargo test -p yoctui-bitbake pty_runner
```
