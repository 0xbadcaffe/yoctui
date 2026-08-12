# Current Task

## Task

**ID:** UX-POPUP-EDITOR-CONFIG-001
**Title:** Migrate configuration and BBMASK popups to shared editor state
**Status:** IN_PROGRESS

## Objective

Use the shared reducer-owned cursor, selection, navigation, and clipboard
behavior for configuration and BBMASK TOML popups without weakening allowlisted
configuration writes or BBMASK confirmation.

## Verification

```bash
cargo test -p yoctui-model config_
cargo test -p yoctui-ui config_
cargo check -p yoctui
```
