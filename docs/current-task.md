# Current Task

## Task

**ID:** UX-POPUP-SDK-PUBLISH-001
**Title:** Move SDK publication draft into a TOML popup
**Status:** IN_PROGRESS

## Objective

Migrate SDK publication destination editing to a bounded vi-style TOML popup
without weakening its typed preview or confirmation.

## Verification

```bash
cargo test -p yoctui-model sdk
cargo test -p yoctui-ui sdk
```
