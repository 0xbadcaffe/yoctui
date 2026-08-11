# Current Task

## Task

**ID:** UX-POPUP-BUILD-001
**Title:** Move build-target and Wic drafts into TOML popups
**Status:** IN_PROGRESS

## Objective

Migrate the build target and Wic editable drafts to bounded vi-style TOML
popups without weakening validation, previews, or destructive confirmations.

## Verification

```bash
cargo test -p yoctui-model build_target
cargo test -p yoctui-model wic
cargo test -p yoctui-ui build_target
cargo test -p yoctui-ui wic
```
