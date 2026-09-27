# Current Task

**ID:** HARDWARE-PDF-VIEWER-001
**Title:** Render readable Hardware documents with usable navigation
**Status:** IN_PROGRESS

v0.1.242 reduces PDF pages to a 320×240 raster before display and enlarges
those discarded pixels when zooming. Its full-body viewer also removes the
Navigator. Retain a bounded high-resolution page, fit/resample it in the UI,
keep Navigator focus usable beside the document, and preserve narrow-terminal
access through the pane switcher.

```bash
cargo test -p yoctui-model hardware
cargo test -p yoctui-app hardware
cargo test -p yoctui-ui hardware
cargo test -p yoctui --bin yoctui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```

Then complete HARDWARE-PDF-VIEWER-RELEASE-001 for v0.1.243. The blocked U-Boot
provider validation and M67 real-Poky evidence remain independent. Full
workspace tests remain deferred until the user requests them.
