# Current Task

## Task

**ID:** UX-POPUP-EDITOR-002
**Title:** Adopt a reusable bounded popup text editor
**Status:** IN_PROGRESS

## Objective

Introduce the reusable editor boundary and select/adapt `tui-textarea` rather
than maintaining append-only per-dialog fields.

## Verification

```bash
cargo test -p yoctui-model popup_editor
cargo test -p yoctui-ui popup_editor
cargo check -p yoctui
```
