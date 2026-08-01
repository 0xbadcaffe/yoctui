# Current Task

## Task

**ID:** MAINT-RELEASE-LOCKED-UI-001
**Title:** Add typed locked-signature cache form

## Objective

Expose a model-owned, focus-trapped entry form for exact
`gen-lockedsig-cache` inputs without duplicating adapter validation or starting
a process.

## Required work

1. Map Release shortcut `l` only when locked-cache capability and authoritative
   native-LSB metadata are available.
2. Add a bounded typed draft for locked-signature include, input cache, output
   cache, read-only native LSB, and optional filter, with deterministic field
   traversal and exact `LockedSignatureCacheRequest` validation.
3. Emit a typed preview effect only on valid `Enter`; `Esc` closes without an
   effect and other pane shortcuts cannot leak through the dialog.
4. Document exact controls and destructive replacement meaning in
   `docs/ui-spec.md`; render authoritative context, selected field, validation,
   and output replacement warning safely at 80x24 and responsive boundaries.
5. Add reducer, app-input, and Ratatui `TestBackend` tests for normal entry,
   invalid input, unavailable capability/context, focus traversal,
   cancellation, bounded text, and narrow rendering.

## Definition of done

- `l` opens only the typed locked-cache form with authoritative native-LSB
  context.
- Valid submission emits one exact request for adapter preview and never
  spawns; invalid submission remains visible.
- Replacement risk is visible and focus is trapped at every supported size.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model maintenance_release_locked_workspace
cargo test -p yoctui-app maintenance_release_locked_workspace
cargo test -p yoctui-ui maintenance_release_locked_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` in the implementation commit with exact controls.
- Update `docs/architecture.md` only if component ownership changes.
- Mark `MAINT-RELEASE-LOCKED-UI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-RELEASE-HISTORY-UI-001`.

## Next task

`MAINT-RELEASE-HISTORY-UI-001`
