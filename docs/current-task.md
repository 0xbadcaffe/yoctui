# Current Task

## Task

**ID:** UX-POPUP-SDK-001
**Title:** Move SDK editable drafts into TOML popups
**Status:** IN_PROGRESS

## Objective

Migrate SDK native and publication draft forms to bounded vi-style TOML popups
without weakening typed previews or validation.

## Verification

```bash
cargo test -p yoctui-model sdk
cargo test -p yoctui-ui sdk
```
