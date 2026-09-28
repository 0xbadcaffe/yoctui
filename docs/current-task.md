# Current Task

**ID:** HARDWARE-PDF-AUTO-XTERM-001
**Title:** Open an automatic readable graphics terminal
**Status:** IN_PROGRESS

When an interactive invocation lacks native graphics, launch the current
executable and original arguments directly through XTerm with VT340, a
14-point scalable Monospace font, and 140x40 cells. Preserve the initialized
environment and prevent recursive relaunch. Continue in the original terminal
when no graphical display or XTerm exists.

```bash
cargo test -p yoctui --bin yoctui terminal_graphics
cargo test -p yoctui-ui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```

Invoke `yoctui attach` from Terminator and inspect the automatically opened
XTerm Hardware library and native PDF before completing the task.
