# Current Task

**ID:** REDUCE-PROTOCOL-SUPPORT-001
**Title:** Decompose protocol transport and persistence support
**Status:** NOT_STARTED

Dependency REDUCE-PROTOCOL-DAEMON-STATE-001 is DONE. Split `daemon_ipc.rs` and
`daemon_persist.rs` production responsibilities into meaningful sources of
approximately 500 lines or less, then audit `lib.rs`, lifecycle, rootfs and
archive production code. Preserve platform gates, permissions and recovery
behavior.

```bash
cargo test -p yoctui-protocol --all-features
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 remains BLOCKED on genuine current-source real-Poky
performance evidence. Preserve historical captures and the running user build.
