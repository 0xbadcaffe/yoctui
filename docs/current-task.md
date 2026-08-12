# Current Task

## Task

**ID:** UX-POPUP-EDITOR-SDK-NATIVE-001
**Title:** Migrate SDK native tools popup to shared editor state
**Status:** IN_PROGRESS

## Objective

Adopt shared reducer-owned multi-field editing for SDK native tools without
weakening FindSysroot versus RunNative restrictions, exact previews, or
confirmation.

## Verification

```bash
cargo test -p yoctui-model sdk_
cargo test -p yoctui-ui sdk_
cargo test -p yoctui sdk_workflow_cli_
cargo check -p yoctui
```
