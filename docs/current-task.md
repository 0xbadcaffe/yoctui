# Current Task

## Task

**ID:** UX-POPUP-OPS-001
**Title:** Move QA, security, and maintenance editable drafts into TOML popups
**Status:** IN_PROGRESS

## Objective

Migrate operational editable drafts to bounded TOML popups while retaining
their exact typed validation, preview, review, and confirmation semantics.

## Verification

```bash
cargo test -p yoctui-model qa
cargo test -p yoctui-model security
cargo test -p yoctui-model maintenance
cargo test -p yoctui-ui qa
cargo test -p yoctui-ui security
cargo test -p yoctui-ui maintenance
```
