# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-LOCKED-001
**Title:** Move Maintenance locked-cache form into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate locked-signature cache inputs and filter editing to the shared bounded
TOML popup without weakening changed-evidence checks or exact preview and
destructive confirmation gates.

## Verification

```bash
cargo test -p yoctui-model maintenance_release
cargo test -p yoctui-app maintenance_release_locked
cargo test -p yoctui-ui maintenance_release_locked
cargo check -p yoctui
```
