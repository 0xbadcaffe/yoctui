# Current Task

## Task

**ID:** SHELL-TERM-001
**Title:** Embed a terminal-emulation state machine
**Status:** NOT_STARTED

## Objective

Embed a bounded VT parser/emulator supporting cursor movement, erase,
attributes, alternate screen, Unicode width, bracketed paste, and resize.

## Verification

```bash
cargo test -p yoctui-shell terminal_emulation
./scripts/test-terminal-corpus.sh
```

## Definition of done

- Terminal emulation handles the documented control sequences safely and keeps
  bounded screen/scrollback state.

## Next task

After completion, select `SHELL-UI-001`.
