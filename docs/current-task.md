# Current Task

**ID:** HARDWARE-PDF-WHEEL-001
**Title:** Navigate PDF pages with the mouse wheel
**Status:** IN_PROGRESS

Map wheel input over an open Hardware PDF to previous/next-page actions and
submit their typed load effects through the existing Hardware worker. Retain
arrow and h/j/k/l panning within the current page.

```bash
cargo test -p yoctui-app hardware
cargo test -p yoctui-ui hardware
cargo test -p yoctui --bin yoctui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```

Validate wheel movement from page 1 to page 2 in a real XTerm native PDF view.
