# Current Task

## Task

**ID:** CLIENT-RUNTIME-QEMU-001
**Title:** Move QEMU job ownership into the daemon
**Status:** IN_PROGRESS

## Objective

Extend the bounded protocol with a closed typed QEMU launch operation. Route
confirmed QEMU effects through correlated daemon requests and sequenced events,
reuse the current validated capability/command runner under daemon ownership,
preserve display/network/device confirmation rules, and ensure client detach
never terminates an active emulator session.

## Verification

```bash
cargo test -p yoctui client_runtime_qemu
```
