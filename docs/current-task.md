# Current Task

**ID:** REDUCE-APP-DAEMON-JOBS-001
**Title:** Decompose application daemon and job coordination
**Status:** NOT_STARTED

Dependency REDUCE-APP-MAPPING-001 is DONE. Split `daemon_client.rs` and
`job_coordinators.rs` into meaningful responsibility files of approximately
500 lines or less, then audit every application production source. Preserve
request correlation, cancellation, typed outcomes and public APIs.

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
