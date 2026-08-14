# Current Task

## Task

**ID:** KEYBOARD-PREFIX-RUNTIME-001
**Title:** Route prefix commands to session and layout actions
**Status:** IN_PROGRESS

## Objective

Execute create/switch/close/split/focus/resize/detach/take-control commands
through typed client effects and daemon requests; no notification-only
placeholders.

## Verification

```bash
cargo test -p yoctui --test keyboard_prefix_runtime keyboard_prefix
```
