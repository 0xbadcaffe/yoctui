# Current Task

## Task

**ID:** UX-POPUP-EDITOR-002
**Title:** Add shared bounded TOML popup editing controls
**Status:** IN_PROGRESS

## Objective

Add model-owned cursor/selection, Home/End, replacement-on-edit, paste, copy,
and a persistent shortcut row for every bounded TOML popup.

## Verification

```bash
cargo test -p yoctui-model popup_editor
cargo test -p yoctui-ui popup_editor
cargo check -p yoctui
```
