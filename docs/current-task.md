# Current Task

**ID:** REDUCE-SHELL-001
**Title:** Finish responsibility-based modules and test folders in yoctui-shell
**Status:** NOT_STARTED

Dependency REDUCE-UI-001 is DONE. Audit `yoctui-shell`, split oversized sources
into meaningful responsibility files of approximately 500 lines or less and
move inline test bodies under test folders. Preserve behavior, public APIs,
assertions and platform gates.

```bash
cargo test -p yoctui-shell --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
