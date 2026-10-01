# Current Task

**ID:** QEMU-GDB-SESSION-001
**Title:** Validate and supervise a private QEMU-to-GDB session
**Status:** IN_PROGRESS

Dependency KERNEL-DEBUG-UI-001 is DONE. Implement only the typed plan, non-spawning
file/symbol/config checks and private managed QEMU/GDB backend, with bounded
readiness/logs and owned-process cleanup. Relevant files: model qemu_debug module,
CLI helper/runtime and tests. Update UI/architecture/roadmap/status/registry, then
advance immediately to QEMU-GDB-UI-001 after the coherent commit.

```bash
cargo test -p yoctui-model qemu_debug
cargo test -p yoctui --bin yoctui qemu_debug
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
./scripts/verify-roadmap.sh
```

Normal/failure fake-process coverage is required; genuine Linux guest evidence
is tracked separately in QEMU-GDB-LIVE-001. No existing daemon restart, kernel
configuration/deployment change or full suite. Preserve the M67 blocker.
