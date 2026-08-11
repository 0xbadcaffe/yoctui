# Current Task

## Task

**ID:** UX-POPUP-WORKFLOWS-001
**Title:** Migrate remaining editable workflow dialogs to popups
**Status:** IN_PROGRESS

## Objective

Migrate remaining editable typed workflow drafts to bounded vi-style TOML
popups without weakening validation or explicit confirmations.

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
