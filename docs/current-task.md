# Current Task

**ID:** HARDWARE-NATIVE-GRAPHICS-001
**Title:** Render readable Hardware PDFs and restore library exit
**Status:** IN_PROGRESS

Implement bounded capability-gated SIXEL presentation for Hardware rasters,
use extracted PDF text when native graphics are unavailable, add explicit
page/text switching, and route `Esc` back to the retained Hardware library from
either visible pane. Do not mutate terminal settings or emit native image bytes
without affirmative capability evidence.

```bash
cargo test -p yoctui-model hardware
cargo test -p yoctui-app hardware
cargo test -p yoctui-ui hardware
cargo test -p yoctui --bin yoctui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
