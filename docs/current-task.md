# Current Task

**ID:** REDUCE-CLI-001
**Title:** Finish responsibility-based modules and test folders in yoctui-cli
**Status:** NOT_STARTED

Dependency REDUCE-CLI-DAEMON-PTY-001 is DONE. Audit `crates/yoctui-cli`, update
source-checker paths, and complete any remaining responsibility-based source
splits or inline-test moves needed to keep source files approximately 500 lines.
Preserve behavior, public APIs, assertions and platform gates. Split further
work into atomic file/family tasks before implementation when needed.

```bash
cargo test -p yoctui --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
