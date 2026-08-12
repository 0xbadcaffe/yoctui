# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-READINESS-001
**Title:** Move Maintenance sstate readiness form into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate Maintenance readiness mode, recipes, mirrors, and timeout editing to a
shared bounded TOML popup without weakening the exact capability-derived
preview.

## Verification

```bash
cargo test -p yoctui-model maintenance_sstate
cargo test -p yoctui-app maintenance_sstate
cargo test -p yoctui-ui maintenance_sstate
cargo check -p yoctui
```
