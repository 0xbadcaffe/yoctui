# Current Task

**ID:** HARDWARE-PDF-NATIVE-ONLY-001
**Title:** Require readable native PDF pages
**Status:** IN_PROGRESS

Implement M95 and validate native screenshots for all five smarc PDFs.

```bash
cargo test -p yoctui --bin yoctui terminal_graphics
cargo test -p yoctui-ui hardware
cargo test -p yoctui --bin yoctui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```

Full suite deferred per user. Release v0.1.248 after focused verification.
