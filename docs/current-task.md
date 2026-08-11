# Current Task

## Task

**ID:** UX-POPUP-CONFIG-001
**Title:** Move configuration and BBMASK editing into TOML popups
**Status:** IN_PROGRESS

## Objective

Replace inline configuration and BBMASK editing with bounded vi-style TOML
popups while retaining their existing validation and confirmation rules.

## Verification

```bash
cargo test -p yoctui-model config_edit
cargo test -p yoctui-ui config
```
