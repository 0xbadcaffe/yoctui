# Current task

## Active task

**ID:** CONFIG-EDIT-PREVIEW-001
**Title:** Add allowlisted configuration edit preview

## Objective

Add a safe, focus-trapping value editor and exact `local.conf` assignment
preview for explicitly allowlisted global configuration variables.

## Required work

1. Inventory existing BBMASK preview/confirmation, variable detail/scope,
   dialog focus, assignment validation, responsive rendering, and tests.
2. Define the initial editable-variable allowlist in typed model code and keep
   all other variables read-only with an exact reason. Recipe-scoped values are
   never directly editable.
3. Add an edit shortcut that opens a value editor prefilled from the exact
   loaded global effective value. Loading, failure, not-loaded, absent value,
   non-allowlisted variable, or recipe scope remains inert.
4. Validate variable name/value as a single assignment value and reject
   newline/control injection.
5. Preview the exact quoted assignment and destination `build/conf/local.conf`
   in a separate confirmation dialog. No write occurs in this task.
6. `Enter` advances editor to preview and preview to a typed write effect;
   `Esc` cancels and restores the exact prior pane.
7. Render shortcut availability and both dialogs across responsive modes.
8. Add reducer, app/input, and TestBackend tests named `config_edit_preview`.
9. Update `docs/ui-spec.md` for allowlist, validation, preview, and focus.

## Definition of done

- Read-only default and allowlist are explicit and tested.
- Exact selected global detail seeds a validated editor.
- Confirmation previews destination and exact assignment before any effect.
- Partial/error/injection states remain inert with precise reasons.
- Focus and responsive rendering are covered.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the write task becomes active.

## Verification

```bash
cargo test -p yoctui-model config_edit_preview
cargo test -p yoctui-app config_edit_preview
cargo test -p yoctui-ui config_edit_preview
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-EDIT-WRITE-001 — Write and refresh previewed configuration edits`
