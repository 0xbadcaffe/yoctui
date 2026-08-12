# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-SERVICE-001
**Title:** Move Maintenance service forms into TOML popups
**Status:** IN_PROGRESS

## Objective

Migrate PR-service import and export drafts to the shared bounded TOML popup
without weakening canonical file checks, side-effect previews, or confirmation.

## Verification

```bash
cargo test -p yoctui-model maintenance_service
cargo test -p yoctui-app maintenance_service
cargo test -p yoctui-ui maintenance_service
cargo check -p yoctui
```
