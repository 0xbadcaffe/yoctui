# Current Task

## Task

**ID:** COMPAT-UI-ACTIONS-001
**Title:** Apply capability state to visible workspace actions
**Status:** IN_PROGRESS

## Objective

Close the aggregate visible-action gate across Navigator, command palette,
workspace tables, Inspectors, dialogs, and footers.

## Dependencies

- `COMPAT-UI-ACTION-CATALOG-001` — DONE
- `COMPAT-UI-NAV-ACTIONS-001` — DONE
- `COMPAT-UI-WORKSPACE-ACTIONS-001` — DONE
- `COMPAT-UI-DIALOG-ACTIONS-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_ui.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Every useful environment-backed action in Navigator, command palette,
  workspace, Inspector, dialog, and footer surfaces uses one typed projection.
- All five states, exact reasons, limitations, and selected maintained
  alternatives remain visible and consistent across those surfaces.
- Disabled operations cannot prepare dialogs or emit effects; limited actions
  remain usable through their selected implementation.
- Client-local navigation, inspection, editors, copy/open, settings, help,
  quit, and owned-process cancellation remain usable without authority.
- Live replacement/invalidation updates every surface without stale widget
  state, local release checks, or a second capability cache.

## Verification

```bash
cargo test -p yoctui-ui compatibility_ui_actions
cargo test -p yoctui-app compatibility_ui_actions
./scripts/test-tui-snapshots.sh
./scripts/verify-roadmap.sh
```
