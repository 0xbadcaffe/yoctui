# Current Task

## Task

**ID:** UX-POPUP-EDITOR-TEST-RESULTS-001
**Title:** Migrate Testing import and comparison popups to shared editor state
**Status:** IN_PROGRESS

## Objective

Adopt shared reducer-owned editing for Testing result import and comparison
while preserving exact identity resolution, bounded typed validation, and
comparison confirmation.

## Verification

```bash
cargo test -p yoctui-model test_results
cargo test -p yoctui-ui test_workflow
cargo check -p yoctui
```
