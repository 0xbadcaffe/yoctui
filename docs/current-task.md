# Current Task

## Task

**ID:** UX-POPUP-EDITOR-003
**Title:** Add typed popup-editor state and field selection
**Status:** IN_PROGRESS

## Objective

Model a shared editor document, cursor, selection, edit mode, field-value
replacement, and bounded undo history without exposing widget state to workflows.

## Verification

```bash
cargo test -p yoctui-model popup_editor
cargo test -p yoctui-app popup_editor
```
