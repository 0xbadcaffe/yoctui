# Current Task

## Task

**ID:** UX-POPUP-EDITOR-SDK-PUBLISH-001
**Title:** Migrate SDK publication popup to shared editor state
**Status:** IN_PROGRESS

## Objective

Adopt selected destination replacement and shared reducer-owned navigation and
clipboard behavior while retaining exact SDK publication validation and
confirmation.

## Verification

```bash
cargo test -p yoctui-model sdk_
cargo test -p yoctui-ui sdk_
cargo test -p yoctui sdk_workflow_cli_
cargo check -p yoctui
```
