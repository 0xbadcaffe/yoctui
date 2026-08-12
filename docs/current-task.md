# Current Task

## Task

**ID:** UX-POPUP-EDITOR-BUILD-001
**Title:** Migrate build environment and clone popups to shared editor state
**Status:** IN_PROGRESS

## Objective

Replace append-only build environment and clone TOML dialog state with the
shared reducer-owned editor while preserving typed profile validation, clone
review, and environment verification.

## Verification

```bash
cargo test -p yoctui-model build_environment
cargo test -p yoctui-ui build_environment
cargo check -p yoctui
```
