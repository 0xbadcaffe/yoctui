# Current Task

## Task

**ID:** UX-POPUP-EDITOR-TEST-LAUNCH-001
**Title:** Migrate Testing launch popup to shared editor state
**Status:** IN_PROGRESS

## Objective

Adopt shared reducer-owned editing for Testing launch fields while preserving
authoritative machine/distro/image context, typed policy validation, and launch
confirmation.

## Verification

```bash
cargo test -p yoctui-model test_workflow
cargo test -p yoctui-ui test_workflow
cargo check -p yoctui
```
