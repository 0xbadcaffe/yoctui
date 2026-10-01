# Current Task

**ID:** HARDWARE-PROJECT-UI-001
**Title:** Expose Hardware projects, restricted previews and manual bring-up controls
**Status:** IN_PROGRESS

Dependency HARDWARE-PROJECT-STORE-001 is DONE. Relevant files: model project
reducer, app input, Hardware renderers, CLI lifecycle routing and document adapter.
Implement specified project/folder/import forms, restricted embedded preview and
manual stage controls. Done requires focused reducer/input/TestBackend/adapter
tests, restart smoke, documentation, version bump, commit/push and optimized
source-bound release binary. Afterwards restore the external M67 blocker.

```bash
cargo test -p yoctui-model hardware_project
cargo test -p yoctui-app hardware
cargo test -p yoctui-ui hardware
cargo test -p yoctui --bin yoctui hardware
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
./scripts/verify-roadmap.sh
python3 scripts/check-version-bump.py
cargo build --release -p yoctui --bin yoctui
```

Full suite remains deferred per user. M67 stays externally blocked.
