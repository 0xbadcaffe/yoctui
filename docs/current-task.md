# Current Task

## Task

**ID:** UX-POPUP-TEST-LAUNCH-001
**Title:** Move test launch draft into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate the test launch form to a bounded vi-style TOML popup without weakening
typed selection, validation, preview, or confirmation.

## Verification

```bash
cargo test -p yoctui-model test_launch
cargo test -p yoctui-ui test_launch
```
