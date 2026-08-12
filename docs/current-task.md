# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-CLEANUP-001
**Title:** Move Maintenance sstate cleanup form into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate Maintenance cleanup scope and toggles to a shared bounded TOML popup
without weakening exact candidate discovery or the destructive phrase and
confirmation sequence.

## Verification

```bash
cargo test -p yoctui-model maintenance_sstate
cargo test -p yoctui-app maintenance_sstate
cargo test -p yoctui-ui maintenance_sstate
cargo check -p yoctui
```
