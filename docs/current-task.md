# Current Task

## Task

**ID:** RAW-COMMAND-UI-001
**Title:** Implement Raw command list
**Status:** IN_PROGRESS

## Objective

Replace the Raw command-column placeholder with a bounded typed list of the
exact catalog entries visible for the selected category or active search.

## Dependencies

- `RAW-CATEGORY-UI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The command column consumes `RawModeState::visible_commands` and renders the
  exact catalog template/identity for the selected category; Favorites uses
  the retained stable-ID order and search uses the reducer's exact projection.
- Every row has textual favorite state and current five-state capability
  availability without inferring support in the widget.
- Executable and reference-only entries remain visibly distinct, selectable,
  and bounded; disabled/reference entries do not become runnable.
- Up/Down and `j`/`k` select exact stable command identities; Left/`h` returns
  to categories and Right/`l`/Enter preserve the command-column state.
- Empty, first, last, long Unicode, large result, and catalog-replacement states
  show explicit bounded position and never panic.
- Wide/medium render categories and commands together; narrow renders only the
  active column with explicit back/forward text and preserves selection on
  resize.
- TestBackend tests cover exact labels/templates, favorite/capability markers,
  reference-only meaning, bounds, no-color accessibility, and responsive state.

## Verification

```bash
cargo test -p yoctui-ui raw_command_list
cargo clippy -p yoctui-model -p yoctui-app -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
