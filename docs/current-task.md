# Current Task

## Task

**ID:** UX-POPUP-SDK-NATIVE-001
**Title:** Move SDK native-tool draft into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate SDK native-tool editing to a bounded vi-style TOML popup without
weakening mode-specific typed validation or confirmation.

## Verification

```bash
cargo test -p yoctui-model sdk
cargo test -p yoctui-ui sdk
```
