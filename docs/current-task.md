# Current Task

## Task

**ID:** UX-POPUP-EDITOR-WIC-001
**Title:** Migrate Wic creation popup to shared editor state
**Status:** IN_PROGRESS

## Objective

Adopt shared reducer-owned editing for Wic creation TOML while preserving typed
kickstart, output, compression, preview, and confirmation semantics.

## Verification

```bash
cargo test -p yoctui-model wic_
cargo test -p yoctui-ui wic_
cargo check -p yoctui
```
