# Current Task

**ID:** REDUCE-CLI-DAEMON-RAW-001
**Title:** Decompose raw daemon execution
**Status:** NOT_STARTED

Dependency REDUCE-CLI-DAEMON-ROOTFS-001 is DONE. Split daemon_raw.rs into named
job, output-attachment, PTY and history supervisor modules targeting
approximately 500 lines each. Move its inline tests into the CLI test folder
without changing exact identity or lifecycle behavior.

```bash
cargo test -p yoctui --all-features daemon_raw
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
