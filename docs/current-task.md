# Current Task

**ID:** REDUCE-CLI-001
**Title:** Finish responsibility-based modules and test folders in yoctui-cli
**Status:** NOT_STARTED

Dependencies REDUCE-CLI-MAIN-001, REDUCE-CLI-DAEMON-001 and
REDUCE-CLI-TUI-001 are DONE. Review the remaining yoctui-cli sources and split
this broad task into atomic file-family tasks before implementation. Current
files above the approximate 500-line target include maintenance CLI, client
runtime/transport, daemon backend families and PTY integration. Preserve public
interfaces, platform gates, runtime behavior and every existing assertion; move
remaining inline test bodies into test folders.

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
