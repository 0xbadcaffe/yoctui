# Current Task

## Task

**ID:** UX-POPUP-TEST-001
**Title:** Move testing editable drafts into TOML popups
**Status:** IN_PROGRESS

## Objective

Migrate test launch, import, comparison, and export draft forms to bounded
vi-style TOML popups without weakening review and confirmation steps.

## Verification

```bash
cargo test -p yoctui-model test_
cargo test -p yoctui-ui test_
```
