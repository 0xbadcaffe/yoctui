# Current Task

## Task

**ID:** RAW-FORM-UI-001
**Title:** Implement Raw command parameter form
**Status:** IN_PROGRESS

## Objective

Render the reducer-owned Raw command form as a focus-trapping, responsive
typed dialog that validates fields and advances only to the existing exact
native-argv preview.

## Dependencies

- `RAW-HELP-UI-001` — DONE
- `RAW-RECIPE-001` — DONE
- `RAW-PREVIEW-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Enter on an enabled executable command opens a `Run BitBake Command` dialog
  bound to the exact catalog command, capability generation, and authoritative
  build directory; disabled and reference-only selections cannot open it.
- The dialog renders the immutable exact command template, declared typed
  fields in catalog order, selector/manual-entry authority, current value,
  inline validation, and the shared bounded Additional arguments editor.
- Field movement, selector choice, manual editing, validation, and
  Normal/Insert popup-editor behavior route through typed reducer actions;
  dialogs trap focus and `Esc`/`q` closes without an execution effect.
- Enter validates the current document and opens the separate exact preview
  only when all required fields and additional argv are valid; errors remain
  inline and no execution begins from the form.
- Authority/catalog replacement, build identity loss, and stale form identity
  close or refresh the form exactly as specified, restore pane focus, explain
  the reason, and emit no start effect.
- The form remains bounded at every supported breakpoint, including `80x24`,
  and preserves its title, immutable template, selected field, validation,
  preview action, and close hint with textual no-color meaning.
- Model, app routing, and TestBackend tests cover every parameter kind,
  selector/manual/empty/error paths, additional argv, focus trapping,
  preview transition, stale authority, resize, Unicode boundaries, and
  no-color accessibility.

## Verification

```bash
cargo test -p yoctui-model raw_form
cargo test -p yoctui-app raw_form
cargo test -p yoctui-ui raw_form
cargo clippy -p yoctui-model -p yoctui-app -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
