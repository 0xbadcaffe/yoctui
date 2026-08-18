# Current Task

## Task

**ID:** COMPAT-UI-WORKSPACE-ACTIONS-001
**Title:** Render capability state in workspace and Inspector actions
**Status:** IN_PROGRESS

## Objective

Render concise centralized compatibility state for every useful
environment-backed action in workspace tables/action lists and persistent
Inspectors, with exact reasons and maintained alternatives discoverable.

## Dependencies

- `COMPAT-UI-MODEL-001` — DONE
- `COMPAT-UI-INSPECTOR-001` — DONE
- `COMPAT-UI-ACTION-CATALOG-001` — DONE
- `COMPAT-UI-NAV-ACTIONS-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_ui.rs`
- `crates/yoctui-model/src/compatibility_ui.rs`
- `crates/yoctui-model/src/workspace_compatibility.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Every useful environment-backed action row or action-list entry derives its
  five-state presentation from the typed action-surface requirement.
- Disabled entries remain visible/selectable for inspection, show an exact
  reason, and cannot emit an effect; limited entries show their selected
  fallback/limitation and remain usable.
- Persistent Inspectors expose full reasons, requirements, and maintained
  implementation alternatives without release-version clutter.
- Client-local copy/open/navigation actions and cancellation of already-owned
  work remain available when environment authority is absent.
- Snapshot replacement updates visible state/reasons without widget caches,
  stale selections, invalid launches, or panics at supported terminal sizes.
- Exhaustive focused tests cover all workspace families and all five states.

## Verification

```bash
cargo test -p yoctui-ui compatibility_ui_workspace_actions
cargo test -p yoctui-app compatibility_ui_workspace_actions
./scripts/test-tui-snapshots.sh
./scripts/verify-roadmap.sh
```
