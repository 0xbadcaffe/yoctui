# Current Task

## Task

**ID:** ENV-CLI-001
**Title:** Start unconfigured sessions and verify managed BitBake connections
**Status:** NOT_STARTED

## Objective

Make build-dir optional for interactive startup, execute onboarding effects,
retain child environments only in the managed session, and install a backend
only after typed verification.

## Verification

```bash
cargo test -p yoctui build_environment
./scripts/test-cli.sh
```

## Definition of done

- No-argument startup creates an unconfigured app instead of using cwd.
- Typed onboarding effects execute and connection success installs the backend.
- CLI and process failures preserve the setup state and disabled build actions.

## Next task

Implement `ENV-CLI-001`.

## Terminal handoff

`ENV-UI-001` completed; `ENV-CLI-001` is eligible to start.
