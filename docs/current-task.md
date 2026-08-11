# Current Task

## Task

**ID:** UX-POPUP-TEST-JUNIT-001
**Title:** Move JUnit export into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate JUnit export destination editing to a bounded vi-style TOML popup
without weakening typed validation or confirmation.

## Verification

```bash
cargo test -p yoctui-model test_
cargo test -p yoctui-ui test_
```
