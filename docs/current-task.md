# Current Task

**ID:** KERNEL-DEBUG-UI-001
**Title:** Expose Kernel debugging tab, forms, guides and terminal launches
**Status:** IN_PROGRESS

Dependency KERNEL-DEBUG-TOOLS-001 is DONE. Relevant files: model debugging
state/reducer/dialog, app controls, Kernel/UI forms, CLI worker/runtime routing.
Done requires the third Kernel tab, typed forms and guides, correlated background
discovery/preparation, exact terminal chooser, focused normal/failure/regression
checks, harmless PTY smoke, version bump, commit/push and source-bound optimized
release binary. Update UI spec, architecture, roadmap, status and registry, then
restore the externally blocked M67 task.

```bash
cargo test -p yoctui-model kernel_debug
cargo test -p yoctui-app kernel_debug
cargo test -p yoctui-ui kernel_debug
cargo test -p yoctui --bin yoctui kernel_debug
cargo test -p yoctui --bin yoctui platform
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
./scripts/verify-roadmap.sh
python3 scripts/check-version-bump.py
cargo build --release -p yoctui --bin yoctui
```

Full suite remains deferred per user.
