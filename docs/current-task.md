# Current Task

**ID:** TERMINAL-DEC-GRAPHICS-001
**Title:** Render DEC Special Graphics in embedded terminals
**Status:** IN_PROGRESS

Implement bounded, chunk-stable VT100 G0/G1 designation and SI/SO selection in
the model-owned terminal emulator so ncurses borders render as Unicode cells
instead of `q`, `x`, `l`, and `m`. Preserve CSI, OSC, DCS, UTF-8, ordinary ASCII,
styles, and the typed protocol/UI boundary.

```bash
cargo test -p yoctui-model terminal_emulation
cargo test -p yoctui --bin yoctui daemon_pty
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
