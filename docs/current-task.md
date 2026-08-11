# Current Task

## Task

**ID:** UX-CLONE-REVIEW-001
**Title:** Implement in-app Poky clone review and initialization
**Status:** IN_PROGRESS

## Objective

Add the in-app clone/review workflow described by the UI specification.

## Verification

```bash
cargo test -p yoctui-model build_environment_clone
cargo test -p yoctui-bitbake poky_clone
```

## Next task

Implement `UX-CLONE-REVIEW-001`.
