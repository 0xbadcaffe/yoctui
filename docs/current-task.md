# Current Task

## Task

**ID:** ENV-MODEL-001
**Title:** Model build environment profiles and gated connection state
**Status:** NOT_STARTED

## Objective

Add pure typed state and reducer behavior for selecting a profile and requiring
a verified connection before build-capable actions can run.

## Verification

```bash
cargo test -p yoctui-model build_environment
```

## Definition of done

- Profile drafts validate source/build identities and lifecycle transitions.
- Build-capable actions remain disabled until a correlated connection succeeds.
- Reducer effects and failure/cancellation paths have focused tests.

## Next task

Implement `ENV-MODEL-001`.

## Terminal handoff

`ENV-GOV-001` completed; `ENV-MODEL-001` is eligible to start.
