# Current Task

## Task

**ID:** UX-POPUP-EDITOR-TARGET-001
**Title:** Migrate build target popup to shared editor state
**Status:** IN_PROGRESS

## Objective

Select and edit the build target TOML value through shared reducer-owned editor
state while retaining the requested task context and explicit build
confirmation.

## Verification

```bash
cargo test -p yoctui-model build_target
cargo test -p yoctui-ui build_target
cargo check -p yoctui
```
