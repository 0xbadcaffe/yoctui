# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-SSTATE-001
**Title:** Move Maintenance sstate forms into TOML popups
**Status:** IN_PROGRESS

## Objective

Verify the readiness and cleanup popup migrations together, including typed
preview boundaries, authoritative cleanup identities, exact candidate
discovery, and destructive cleanup confirmation.

## Verification

```bash
cargo test -p yoctui-model maintenance_sstate
cargo test -p yoctui-app maintenance_sstate
cargo test -p yoctui-ui maintenance_sstate
cargo check -p yoctui
```
