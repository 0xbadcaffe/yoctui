# Current Task

## Task

**ID:** UX-POPUP-EDITOR-004
**Title:** Wire shared popup-editor input and rendering
**Status:** IN_PROGRESS

## Objective

Route Home/End, arrows, vi Normal/Insert actions, bracketed paste, copy, and
visible shortcut hints through one popup renderer and CLI input path.

## Verification

```bash
cargo test -p yoctui-ui popup_editor
cargo check -p yoctui
```
