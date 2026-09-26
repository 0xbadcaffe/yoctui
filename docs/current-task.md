# Current Task

**ID:** HARDWARE-PERSISTENCE-001
**Title:** Persist and load validated Hardware documents
**Status:** IN_PROGRESS

Add bounded local browsing, regular non-symlink file validation, asynchronous
PDF/image/schematic conversion with a searchable text fallback, generation
checks, and atomic private-session persistence/restoration.

Verify with:

```bash
cargo test -p yoctui --bin yoctui hardware
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
