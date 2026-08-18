# Current Task

## Task

**ID:** COMPAT-WORKSPACE-001
**Title:** Apply capabilities to all workspaces
**Status:** IN_PROGRESS

## Objective

Audit every Navigator destination and action so the centralized daemon
capability snapshot controls availability, implementation selection, and
rejection before effects.

## Dependencies

- `COMPAT-UTILITIES-001` — DONE

## Relevant files

- `crates/yoctui-model/src/`
- `crates/yoctui-app/src/`
- `crates/yoctui-ui/src/`
- `crates/yoctui-bitbake/src/`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Recipes, Layers, Configuration, Tasks, Logs, Errors, Dependencies,
  Signatures, Packages, Images, SDK, Testing, Security, QA, Devtool, QEMU/Wic,
  Maintenance, Project Profiles, and Terminal sessions are audited.
- Every capability-dependent workspace action derives enabled/disabled state
  and its exact reason from the same current snapshot.
- Enabled effects use the catalog-selected implementation and unavailable,
  unknown, unsupported, or stale actions are rejected before launch.
- Snapshot changes preserve valid selections and safely revalidate or close
  invalid dialogs without UI-local release/version checks.
- Model and app tests cover full, partial, absent, and changing snapshots and
  prove no invalid effect is emitted.

## Verification

```bash
cargo test -p yoctui-app compatibility_workspace
cargo test -p yoctui-model compatibility_workspace
./scripts/verify-roadmap.sh
```
