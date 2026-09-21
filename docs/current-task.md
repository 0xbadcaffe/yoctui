# Current Task

**ID:** REDUCE-BITBAKE-ARTIFACTS-001
**Title:** Decompose BitBake artifact and metadata adapters
**Status:** NOT_STARTED

Dependency REDUCE-BITBAKE-MAINTENANCE-001 is DONE. Split production
responsibilities in `wic.rs`, `package.rs`, `rootfs.rs` and `signature.rs` into
meaningful sources of approximately 500 lines or less. Preserve public APIs,
filesystem containment, process ownership and typed outcomes.

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
