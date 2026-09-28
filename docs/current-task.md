# Current Task

**ID:** HARDWARE-PDF-DETAIL-001
**Title:** Open PDFs as pages with detailed terminal projection
**Status:** IN_PROGRESS

Make every PDF open in Page presentation while keeping `v` text view. Replace
the unsupported-terminal PDF half-block projection with colored 2×4 Braille
cells, retain the existing raster-image renderer, and validate all 92 pages of
the five PDFs under `~/projects/smarc`.

```bash
cargo test -p yoctui-model hardware
cargo test -p yoctui-ui hardware
cargo test -p yoctui --bin yoctui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```

Full workspace tests remain deferred at the user's request. After the focused
task, release v0.1.247, install it, restart the initialized Romulus daemon, and
push the coherent commits.
