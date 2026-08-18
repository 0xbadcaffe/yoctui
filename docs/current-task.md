# Current Task

## Task

**ID:** COMPAT-UI-DIALOG-ACTIONS-001
**Title:** Render and enforce capability state in dialogs
**Status:** IN_PROGRESS

## Objective

Render the current centralized capability state inside every environment-backed
dialog and ensure confirmation remains correlated to that same authority.

## Dependencies

- `COMPAT-UI-MODEL-001` — DONE
- `COMPAT-UI-INSPECTOR-001` — DONE
- `COMPAT-UI-ACTION-CATALOG-001` — DONE
- `COMPAT-UI-NAV-ACTIONS-001` — DONE
- `COMPAT-UI-WORKSPACE-ACTIONS-001` — DONE

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

- Every environment-backed dialog shows Available, Limited, Unavailable,
  Unknown, or Unsupported from `workspace_dialog_requirement`.
- Limited dialogs remain confirmable and show exact limitations plus the
  selected implementation before confirmation.
- Unavailable/Unknown/Unsupported dialogs show the exact reason and cannot emit
  a confirm effect; their confirmation control is visibly disabled.
- Snapshot replacement revalidates the open dialog, safely closes an invalid
  launch dialog with restored focus/reason, and ignores stale generations.
- Client-local editors, copy/open dialogs, quit, and owned-process cancellation
  remain usable without environment authority.
- Every dialog family and all five states render safely at responsive bounds;
  renderers contain no release/version policy or widget-local cache.

## Verification

```bash
cargo test -p yoctui-ui compatibility_ui_dialog_actions
cargo test -p yoctui-app compatibility_ui_dialog_actions
./scripts/test-tui-snapshots.sh
./scripts/verify-roadmap.sh
```
