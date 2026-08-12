# Current Task

## Task

**ID:** UX-POPUP-TEST-JUNIT-001
**Title:** Move JUnit export into a TOML popup
**Status:** IN_PROGRESS

## Objective

Complete and verify the shared TOML popup path for JUnit export destination
editing without weakening typed destination inspection, non-overwrite policy,
or explicit export confirmation.

## Verification

```bash
cargo test -p yoctui-model test_
cargo test -p yoctui-ui test_
```
