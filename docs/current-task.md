# Current Task

## Task

**ID:** COMPAT-UI-ACTIONS-001
**Title:** Apply capability state to visible workspace actions
**Status:** IN_PROGRESS

## Objective

Make useful environment-backed actions in every existing UI surface visibly
derive their state and exact reason from the centralized typed compatibility
projection, while preserving client-local and owned-process actions.

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

- Useful environment-backed actions remain discoverable and show Available,
  Limited, Unavailable, Unknown, or Unsupported from one model projection.
- Navigator, command palette, workspace tables, Inspectors, dialogs, and
  contextual footers expose concise state and the exact reason where relevant.
- Disabled actions can be inspected but cannot confirm or emit an effect;
  limited actions explain the selected fallback or limitation.
- Client-local navigation/settings/help/copy/open actions and cancellation of
  already-owned processes remain usable without current capability authority.
- Snapshot replacement updates visible action state without widget-local caches,
  stale reasons, invalid command launches, selection loss, or panics.
- Renderers contain no release/version policy and unexplained `Unsupported`
  labels are absent.

## Verification

```bash
cargo test -p yoctui-ui compatibility_ui_actions
cargo test -p yoctui-app compatibility_ui_actions
./scripts/test-tui-snapshots.sh
./scripts/verify-roadmap.sh
```
