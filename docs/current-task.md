# Current Task

**ID:** KERNEL-DEBUG-TOOLS-001
**Title:** Plan and prepare typed Kernel debugging tools
**Status:** IN_PROGRESS

Dependency HARDWARE-PROJECT-UI-001 is DONE. Relevant files: model debugging
catalogue/drafts/reducer and CLI discovery/preparation adapter. Done requires
bounded typed plans, explicit host/SSH scope, safe quoting, GDB startup safety,
regular-file/executable checks, correlated results and focused normal/failure
tests. Required docs: UI spec, architecture, roadmap, status, registry and next
current task. KERNEL-DEBUG-UI-001 follows immediately; M67 stays blocked.

```bash
cargo test -p yoctui-model kernel_debug
cargo test -p yoctui --bin yoctui kernel_debug
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
./scripts/verify-roadmap.sh
```

Full suite remains deferred per user.
