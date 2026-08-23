# Current Task

## Task

**ID:** RAW-FAVORITE-UI-001
**Title:** Implement Raw Favorites browser and actions
**Status:** IN_PROGRESS

## Objective

Provide a complete typed Favorites workspace for inspecting, adding, editing,
ordering, removing, and reopening persistent Raw command configurations.

## Dependencies

- `RAW-FAVORITE-PERSIST-001` — DONE
- `RAW-FORM-UI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The Favorites workspace renders bounded ordered records with name, command
  template, defaults, additional argv, stale state, and exact current five-state
  compatibility reason at supported widths and without color.
- Typed actions add the selected command, rename, edit defaults/argv, reorder,
  inspect, reopen configuration, and remove only after exact confirmation.
- All edits validate before publication and trigger atomic user-local
  persistence without changing project files or daemon state.
- Stale or unavailable favorites remain inspectable and editable but cannot
  open an executable form until current catalog and capability validation pass.
- Reopening a current favorite creates only a fresh form and retains the normal
  exact preview, confirmation, and new-request execution boundary.
- Keyboard focus is explicit, dialogs trap focus, empty/invalid/bounded states
  are clear, and narrow terminals never panic.
- Model, app, and `TestBackend` tests cover the complete actions, persistence
  effects, stale/unavailable states, focus, confirmation, and responsive output.

## Verification

```bash
cargo test -p yoctui-model raw_favorite_ui
cargo test -p yoctui-app raw_favorite_ui
cargo test -p yoctui-ui raw_favorite_ui
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
