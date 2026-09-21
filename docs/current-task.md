# Current Task

**ID:** REDUCE-BITBAKE-RUNNERS-001
**Title:** Decompose BitBake workflow runners
**Status:** NOT_STARTED

Dependency REDUCE-BITBAKE-REPORTS-001 is DONE. Split production responsibilities
in `sdk_tool.rs`, `qa_layer.rs`, `test_runner.rs`, `security_mapper.rs` and
`raw_job.rs` into meaningful sources of approximately 500 lines or less.
Preserve public APIs, platform gates and adapter behavior.

```bash
cargo test -p yoctui-bitbake --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
