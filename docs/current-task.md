# Current Task

## Task

**ID:** ENV-CLONE-001
**Title:** Add reviewed Poky clone setup
**Status:** NOT_STARTED

## Objective

Add exact non-shell clone/checkout previews, empty-destination safeguards,
cancellation, and typed outcomes.

## Verification

```bash
cargo test -p yoctui-bitbake poky_clone
cargo test -p yoctui-model build_environment_clone
```

## Definition of done

- Clone previews contain exact shell-free vectors and require confirmation.
- Nonempty destinations and unsafe revisions are rejected.
- Fake-process tests cover success, cancellation, and failures.

## Next task

Implement `ENV-CLONE-001`.

## Terminal handoff

`ENV-ADAPTER-001` completed; `ENV-CLONE-001` is eligible to start.
