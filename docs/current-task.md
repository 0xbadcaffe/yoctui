# Current Task

## Task

**ID:** UX-POPUP-TEST-RESULTS-001
**Title:** Move test result forms into TOML popups
**Status:** IN_PROGRESS

## Objective

Migrate import, comparison, and JUnit export forms to bounded vi-style TOML
popups without weakening review or confirmation.

## Verification

```bash
cargo test -p yoctui-model test_
cargo test -p yoctui-ui test_
```
