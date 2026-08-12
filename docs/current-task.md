# Current Task

## Task

**ID:** PTY-EMU-001
**Title:** Add maintained terminal-emulation state machine
**Status:** IN_PROGRESS

## Objective

Adopt a maintained ANSI/VT parser and terminal emulator behind a pure typed
model boundary. Feed raw PTY output, expose bounded screen/scrollback snapshots,
resize safely, and support the common cursor, style, alternate-screen,
bracketed-paste and terminal modes required by shells, editors, ncurses,
menuconfig and devshell without ad-hoc escape parsing.

## Verification

```bash
cargo test -p yoctui-model terminal_emulation
```
