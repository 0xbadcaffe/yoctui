# Current Task

## Task

**ID:** ENV-ADAPTER-001
**Title:** Validate and initialize a selected build environment
**Status:** NOT_STARTED

## Objective

Add bounded source/build/script validation and child-only environment
initialization results for the selected profile.

## Verification

```bash
cargo test -p yoctui-bitbake build_environment
```

## Definition of done

- Adapter validation is bounded and rejects unsafe paths before execution.
- Initialization returns child-only environment data or typed
  interactive-required/failure/cancellation results.
- Fake-process tests cover normal and failure paths.

## Next task

Implement `ENV-ADAPTER-001`.

## Terminal handoff

`ENV-MODEL-001` completed; `ENV-ADAPTER-001` is eligible to start.
