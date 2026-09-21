# Current Task

**ID:** REDUCE-LAYOUT-GATE-001
**Title:** Verify the complete source inventory and enforce the module-size convention
**Status:** NOT_STARTED

Dependency REDUCE-TOOLS-001 is DONE. Extend `scripts/check-library-layout.py`
to inventory the complete maintained source tree, enforce the approximately
500-line module convention and reject inline Rust test bodies. Document the
enforced layout boundary in `docs/architecture.md`.

```bash
python3 scripts/check-library-layout.py
python3 -m pytest bridge/tests
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
