# Current Task

## Task

**ID:** UX-POPUP-OPS-QA-001
**Title:** Move QA report import into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate QA report import to the shared bounded TOML popup while retaining
exact scope/report identity, bounded adapter import, and typed validation.

## Verification

```bash
cargo test -p yoctui-model qa
cargo test -p yoctui-app qa
cargo test -p yoctui-ui qa
cargo check -p yoctui
```
