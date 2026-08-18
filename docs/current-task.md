# Current Task

## Task

**ID:** COMPAT-UI-MODEL-001
**Title:** Project typed compatibility presentation state
**Status:** IN_PROGRESS

## Objective

Create the pure bounded model projection that every compatibility-aware widget
and action surface consumes.

## Dependencies

- `COMPAT-WORKSPACE-001` — DONE
- `COMPAT-PROTOCOL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_ui.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- One pure typed projection exposes normalized environment identity, snapshot
  generation, operating mode, and exact counts for all five states.
- Capability rows retain stable ID, state, exact reason/requirement,
  limitations, selected implementation, and bounded evidence.
- Absent authority is explicit Unknown and never derives support from host or
  release values.
- Typed filter, search, selection, and selected-detail state remains valid as
  snapshots load, unload, or change generation.
- Reusable visible-action presentation maps centralized workspace availability
  to enabled, limited, unavailable, unsupported, or unknown with exact reasons.

## Verification

```bash
cargo test -p yoctui-model compatibility_ui_model
cargo test -p yoctui-app compatibility_ui_model
./scripts/verify-roadmap.sh
```
