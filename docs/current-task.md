# Current Task

## Task

**ID:** ENV-CONNECT-001
**Title:** Execute onboarding effects and verify managed BitBake connections
**Status:** NOT_STARTED

## Objective

Execute typed initialization/clone effects, retain child environments only in
the managed session, and install a backend only after a typed workspace
verification succeeds.

## Verification

```bash
cargo test -p yoctui build_environment
./scripts/test-cli.sh
```

## Definition of done

- Setup effects execute without blocking the TUI.
- Backend installation is gated on typed workspace verification.
- Connection failure, cancellation, and loss remain actionable states.

## Next task

Implement `ENV-CONNECT-001`.

## Terminal handoff

`ENV-CLI-001` completed; `ENV-CONNECT-001` is eligible to start.
