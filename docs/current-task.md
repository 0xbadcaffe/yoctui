# Current Task

**ID:** REDUCE-APP-001
**Title:** Finish responsibility-based modules and test folders in yoctui-app
**Status:** NOT_STARTED

Dependency REDUCE-BITBAKE-001 is DONE. Audit `crates/yoctui-app`, split sources
into meaningful responsibility files of approximately 500 lines or less, and
move inline unit-test bodies into descriptive folders under `src/tests`.
Preserve public APIs, platform gates, test names and assertions.

```bash
cargo test -p yoctui-app --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
