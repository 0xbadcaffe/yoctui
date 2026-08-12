# Current Task

## Task

**ID:** CLIENT-RUNTIME-DEVTOOL-001
**Title:** Move Devtool job ownership into the daemon
**Status:** IN_PROGRESS

## Objective

Extend the bounded protocol and daemon runtime with closed typed representations
of the existing Devtool operations. Route Devtool start/cancel effects through
correlated daemon requests and events, run them with the existing adapter/job
infrastructure in the daemon, and remove interactive-client runner ownership so
detach cannot cancel active Devtool work.

## Verification

```bash
cargo test -p yoctui client_runtime_devtool
```
