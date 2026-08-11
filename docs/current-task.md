# Current Task

## Task

**ID:** UX-POPUP-TEST-IMPORT-001
**Title:** Move test result import into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate test result import to a bounded vi-style TOML popup without weakening
typed validation or confirmation.

## Verification

```bash
cargo test -p yoctui-model test_
cargo test -p yoctui-ui test_
```
