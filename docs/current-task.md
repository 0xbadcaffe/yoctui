# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-SSTATE-001
**Title:** Move Maintenance sstate forms into TOML popups
**Status:** IN_PROGRESS

## Objective

Migrate Maintenance readiness and cleanup editable drafts to shared bounded
TOML popups without weakening exact candidate previews or destructive cleanup
phrase confirmation.

## Verification

```bash
cargo test -p yoctui-model maintenance_sstate
cargo test -p yoctui-app maintenance_sstate
cargo test -p yoctui-ui maintenance_sstate
cargo check -p yoctui
```
