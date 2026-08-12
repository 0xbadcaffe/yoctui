# Current Task

## Task

**ID:** UX-POPUP-EDITOR-MIGRATION-001
**Title:** Complete shared popup editor migration
**Status:** IN_PROGRESS

## Objective

Verify the migrated build, configuration, target, Wic, SDK, and Testing
workflows use the shared reducer-owned editor consistently without bypassing
their typed preview, validation, or confirmation gates.

## Verification

```bash
cargo test -p yoctui-model popup_editor
cargo test -p yoctui-ui popup_editor
cargo test --workspace --all-features
cargo check -p yoctui
```
