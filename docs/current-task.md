# Current Task

## Task

**ID:** UX-POPUP-OPS-001
**Title:** Move QA, security, and maintenance editable drafts into TOML popups
**Status:** IN_PROGRESS

## Objective

Verify the Security import, QA import, and all Maintenance shared-popup
migrations together without weakening their typed adapter, confirmation, and
side-effect boundaries.

## Verification

```bash
cargo test -p yoctui-model qa
cargo test -p yoctui-model security
cargo test -p yoctui-model maintenance
cargo test -p yoctui-ui qa
cargo test -p yoctui-ui security
cargo test -p yoctui-ui maintenance
```
