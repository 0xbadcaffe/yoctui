# Current Task

**ID:** REDUCE-CLI-DAEMON-ROOTFS-001
**Title:** Decompose daemon rootfs inspection
**Status:** NOT_STARTED

Dependency REDUCE-CLI-DAEMON-COMPAT-001 is DONE. Split daemon_rootfs.rs into
named client-query, validation, worker and bridge-mapping modules targeting
approximately 500 lines each. Move its inline tests into the CLI test folder
without changing identity or cancellation checks.

```bash
cargo test -p yoctui --all-features daemon_rootfs
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
