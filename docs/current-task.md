# Current Task

## Task

**ID:** COMPAT-UI-ACTION-CATALOG-001
**Title:** Catalog visible UI action capability requirements
**Status:** IN_PROGRESS

## Objective

Create one compiler-checked inventory that maps every visible UI action surface
to client-local behavior or an existing typed workspace capability requirement.

## Dependencies

- `COMPAT-UI-MODEL-001` — DONE
- `COMPAT-UI-INSPECTOR-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_ui.rs`
- `crates/yoctui-model/src/workspace_compatibility.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- A closed typed inventory covers Navigator destinations, command-palette
  operations, contextual workspace actions, and every dialog variant.
- Each action maps centrally to `ClientLocal` or the existing exact all-of,
  any-of, single-capability, or owned-process requirement.
- Projection preserves Available, AvailableWithLimitations, Unavailable,
  Unknown, Unsupported, exact reasons, and selected implementations.
- Missing authority fails environment-backed actions closed while client-local
  actions and cancellation of already-owned processes remain enabled.
- Exhaustive tests fail when a new destination, command, action, or dialog is
  added without a compatibility classification.
- The inventory contains no release/version comparisons or renderer policy.

## Verification

```bash
cargo test -p yoctui-model compatibility_ui_action_catalog
./scripts/verify-roadmap.sh
```
