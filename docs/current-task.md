# Current Task

## Task

**ID:** KEYBOARD-PREFIX-001
**Title:** Add tmux-style prefix keyboard layer
**Status:** IN_PROGRESS

## Objective

Add a configurable documented prefix for terminal-session creation, switching,
splitting, navigation, detach, help, and command-palette actions without
stealing input from the active terminal application.

## Verification

```bash
cargo test -p yoctui-app keyboard_prefix
cargo test -p yoctui-ui keyboard_prefix
```
