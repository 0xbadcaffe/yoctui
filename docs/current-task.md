# Current Task

## Task

**ID:** CLIENT-RUNTIME-SDK-001
**Title:** Move SDK job ownership into the daemon
**Status:** IN_PROGRESS

## Objective

Extend the bounded protocol with closed typed SDK build, publication, and native
operations. Route existing SDK effects through correlated daemon requests and
sequenced events, reuse the current validated adapters/runners under daemon
ownership, preserve exact artifact/environment identities and cancellation,
and ensure client detach never terminates active SDK work.

## Verification

```bash
cargo test -p yoctui client_runtime_sdk
```
