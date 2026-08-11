# Current Task

## Task

**ID:** UX-POPUP-TEST-COMPARE-001
**Title:** Move test comparison into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate comparison selection to a bounded vi-style TOML popup without weakening
typed preview or confirmation.

## Verification

```bash
cargo test -p yoctui-model test_
cargo test -p yoctui-ui test_
```
