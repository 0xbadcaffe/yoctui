# Current Task

## Task

**ID:** UX-POPUP-OPS-SECURITY-001
**Title:** Move Security report import into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate Security report import to the shared bounded TOML popup while
retaining canonical report identity, bounded adapter import, and typed
validation.

## Verification

```bash
cargo test -p yoctui-model security
cargo test -p yoctui-app security
cargo test -p yoctui-ui security
cargo check -p yoctui
```
