# Current Task

## Task

**ID:** DEVWORK-EDITOR-001
**Title:** Make recipe and Devtool source editing language-aware
**Status:** IN_PROGRESS

## Objective

Replace the append-only recipe/Devtool source buffer with the existing bounded
reducer-owned text area and add honest path-derived language awareness without
introducing widget-owned or fabricated compiler state.

## Dependencies

- DEVWORK-GOV-001 — DONE
- UX-TEXTAREA-UI-001 — DONE
- DEVTOOL-MODIFY-001 — DONE

## Definition of done

- Recipe and Devtool workspace files use `TextAreaState` editing semantics.
- Language identity and bounded syntax/structural diagnostics cover the
  contracted source-language set without claiming LSP output.
- Save is canonical-root-contained, conflict-aware, permission-preserving, and
  atomic.
- Build/update/finish retain the exact selected Devtool recipe identity.
- Focused model/app/UI/CLI tests pass.

## Verification

```bash
cargo test -p yoctui-model devwork_editor
cargo test -p yoctui-app devwork_editor
cargo test -p yoctui-ui devwork_editor
cargo test -p yoctui -- devwork_editor
```
