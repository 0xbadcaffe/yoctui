# Current Task

**ID:** HARDWARE-PDF-FALLBACK-001
**Title:** Render readable non-native PDFs and restore library return
**Status:** IN_PROGRESS

Reject control/private-use and symbol-only PDF extraction, open those PDFs in a
fit-width raster presentation on terminals without native graphics, and map both
Escape and Backspace to the typed close-viewer action while retaining the
Hardware category and selection.

```bash
cargo test -p yoctui-model hardware
cargo test -p yoctui-app hardware
cargo test -p yoctui-ui hardware
cargo test -p yoctui --bin yoctui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```

Full workspace tests remain deferred at the user's request. After this focused
task, release v0.1.246, install it, validate the reported document live, restart
the initialized Romulus daemon, commit, and push.
