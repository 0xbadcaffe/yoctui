# Current Task

## Task

**ID:** UX-POPUP-OPS-MAINT-ARCHIVE-001
**Title:** Move Maintenance Git archive form into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate Git release archive paths, toggles, names, messages, exclusions, notes,
and optional push remote to the shared bounded TOML popup while preserving the
separate local archive and network-push confirmation stages.

## Verification

```bash
cargo test -p yoctui-model maintenance_release
cargo test -p yoctui-app maintenance_release_archive
cargo test -p yoctui-ui maintenance_release_archive
cargo check -p yoctui
```
