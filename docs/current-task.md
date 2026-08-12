# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-RELEASE-001
**Title:** Move Maintenance release forms into TOML popups
**Status:** IN_PROGRESS

## Objective

Verify locked-cache, build-history, and Git archive popup migrations together,
including repository evidence, changed-output safeguards, bounded comparisons,
and separate local/network confirmation.

## Verification

```bash
cargo test -p yoctui-model maintenance_release
cargo test -p yoctui-app maintenance_release
cargo test -p yoctui-ui maintenance_release
cargo check -p yoctui
```
