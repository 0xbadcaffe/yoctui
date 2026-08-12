# Current Task

## Task

**ID:** UX-POPUP-EDITOR-INPUT-001
**Title:** Normalize popup editor keyboard and clipboard input
**Status:** IN_PROGRESS

## Objective

Map Home/End, arrows, Ctrl+C, Ctrl+V, bracketed paste, and vi editor commands
into typed client input without stealing terminal-session input.

## Verification

```bash
cargo test -p yoctui input_from_key
cargo check -p yoctui
```
