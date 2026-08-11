# Current Task

## Task

**ID:** UX-POPUP-TARGET-001
**Title:** Move build target draft into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate the build target draft to a bounded vi-style TOML popup without
weakening task selection, validation, or build confirmation.

## Verification

```bash
cargo test -p yoctui-model build_target
cargo test -p yoctui-ui build_target
cargo check -p yoctui
```
