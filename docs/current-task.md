# Current Task

## Task

**ID:** UX-ENV-FORM-001
**Title:** Edit build directory, source environment, and unlock images
**Status:** NOT_STARTED

## Objective

Provide typed source/build/script editing, profile replacement invalidation,
initialization prompts, verified BitBake image inventory, and image build
enablement.

## Verification

```bash
cargo test -p yoctui-model build_environment_form
cargo test -p yoctui-ui build_environment_form
cargo test -p yoctui -- build_environment_form
```

## Definition of done

- Source/build/script fields are editable as typed values.
- Profile replacement invalidates connection and image inventory.
- Verified BitBake exposes available images and enables build actions.

## Next task

Implement `UX-ENV-FORM-001`.

## Terminal handoff

`UX-ENV-NAV-001` completed; `UX-ENV-FORM-001` is eligible to start.
