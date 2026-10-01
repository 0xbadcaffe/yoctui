# Current Task

**ID:** QEMU-GDB-UI-001
**Title:** Launch managed QEMU debugging from the Kernel workbench
**Status:** IN_PROGRESS

Dependency QEMU-GDB-SESSION-001 is DONE. Add the managed technique, trapped
explicit inputs, initialized tool discovery and typed underlying child-command
review using the existing embedded/detached launcher. Preserve normal QEMU and
all existing Kernel techniques. Update specs/status/registry, bump version and
commit, then advance immediately to QEMU-GDB-LIVE-001 before final push/release.

```bash
cargo test -p yoctui-model kernel_debug
cargo test -p yoctui-app kernel_debug
cargo test -p yoctui-ui kernel_debug
cargo test -p yoctui --bin yoctui kernel_debug
cargo test -p yoctui --bin yoctui qemu_workspace
cargo fmt --all --check
cargo clippy -p yoctui --all-targets --all-features -- -D warnings
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
```

Also check menuconfig/client-runtime, saved environment, Hardware and RootFS
regressions. Optimized build follows final source commit. No existing daemon
restart, kernel configuration/deployment change or full suite. Preserve M67.
