# Current Task

**ID:** REDUCE-CLI-MAINTENANCE-001
**Title:** Decompose maintenance workflows and move inline tests
**Status:** NOT_STARTED

Dependency REDUCE-CLI-TUI-001 is DONE. Split maintenance_cli.rs into named
workflow, preview, runner and persistence modules targeting approximately 500
lines each. Move its inline tests into the CLI test folder without changing
assertions, platform behavior, process ownership, cancellation or typed effects.

```bash
cargo test -p yoctui --all-features maintenance
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
