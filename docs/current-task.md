# Current Task

## Task

**ID:** UX-POPUP-EDITOR-005
**Title:** Migrate existing TOML forms onto the shared editor
**Status:** IN_PROGRESS

## Objective

Replace append-only implementation in build environment, clone, configuration,
BBMASK, target, Wic, SDK, and Testing forms while retaining typed parsers,
previews, and confirmations. Split this task before implementation if the
migration cannot fit one coherent commit.

## Verification

```bash
cargo test -p yoctui-model popup_editor
cargo test -p yoctui-ui popup_editor
cargo check -p yoctui
```
