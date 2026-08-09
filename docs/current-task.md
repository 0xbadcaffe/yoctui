# Current Task

## Task

**ID:** ENV-CONNECT-002
**Title:** Install the managed backend after typed verification
**Status:** NOT_STARTED

## Objective

Install a managed bridge/process backend only after the typed workspace
verification succeeds, using the captured child environment.

## Verification

```bash
cargo test -p yoctui build_environment
```

## Definition of done

- Backend installation is gated on typed workspace verification.
- Captured child environment is applied only to managed backend children.
- Connection failure, cancellation, and loss remain actionable states.

## Next task

Implement `ENV-CONNECT-002`.

## Terminal handoff

`ENV-CONNECT-001` completed; `ENV-CONNECT-002` is eligible to start.
