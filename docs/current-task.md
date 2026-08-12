# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-HISTORY-001
**Title:** Move Maintenance build-history form into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate build-history revisions, report choices, exclusions, and colour choice
to the shared bounded TOML popup while retaining the authoritative repository
identity and bounded typed comparison preview.

## Verification

```bash
cargo test -p yoctui-model maintenance_release
cargo test -p yoctui-app maintenance_release_history
cargo test -p yoctui-ui maintenance_release_history
cargo check -p yoctui
```
