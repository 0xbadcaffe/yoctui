# Current Task

**ID:** HARDWARE-PROJECT-STORE-001
**Title:** Persist real Hardware project folders, files and manual stage values
**Status:** IN_PROGRESS

Dependency SAVED-ENV-LOAD-001 is DONE. Relevant files: new model project types,
CLI filesystem project store and focused tests. Definition of done: private
bounded manifests, named project/subfolders, contained navigation, arbitrary
regular-file import without overwrite, restart-safe stage persistence and
normal/error/containment tests. Update architecture, UI specification, registry
and implementation status. Follow immediately with HARDWARE-PROJECT-UI-001.

```bash
cargo test -p yoctui-model hardware_project
cargo test -p yoctui --bin yoctui hardware_project
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
./scripts/verify-roadmap.sh
```

Full suite remains deferred per user. M67 stays externally blocked.
