# Current Task

## Task

**ID:** UX-POPUP-WIC-001
**Title:** Move Wic create draft into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate the Wic create draft to a bounded vi-style TOML popup without
weakening typed choices, preview, or protected device-write confirmation.

## Verification

```bash
cargo test -p yoctui-model wic
cargo test -p yoctui-ui wic
```
