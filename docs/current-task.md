# Current Task

## Task

**ID:** MAINT-RELEASE-HISTORY-UI-001
**Title:** Add typed build-history comparison form

## Objective

Expose a model-owned, focus-trapped `buildhistory-diff` entry form without
emulating the separate unsupported `build-compare` interface or starting a
process.

## Required work

1. Map Release shortcut `h` only when the exact build-history-diff capability
   and canonical authoritative `BUILDHISTORY_DIR` repository are available.
2. Add a bounded typed draft with read-only repository, optional from/to
   revisions, report-version, report-all, signatures, signature-diff,
   exclude-paths, and no-colour choices.
3. Define deterministic field traversal, typed toggles, bounded comma-separated
   exclusions, exact `BuildComparisonRequest` validation, `Enter` preview, and
   side-effect-free `Esc` cancellation.
4. Document exact controls in `docs/ui-spec.md`; render selected fields,
   authoritative repository, validation, and bounded-report meaning safely at
   80x24 and responsive boundaries.
5. Add reducer, app-input, and Ratatui `TestBackend` tests for valid/invalid
   entry, unavailable context, traversal/toggles, cancellation, bounds, narrow
   rendering, and explicit separation from `build-compare`.

## Definition of done

- `h` opens only the typed buildhistory-diff form using the exact configured
  repository.
- Valid submission emits one exact typed preview effect and never spawns.
- Invalid/unavailable requests remain visible or inert as specified.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model maintenance_release_history_workspace
cargo test -p yoctui-app maintenance_release_history_workspace
cargo test -p yoctui-ui maintenance_release_history_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` in the implementation commit with exact controls.
- Update `docs/architecture.md` only if component ownership changes.
- Mark `MAINT-RELEASE-HISTORY-UI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-RELEASE-ARCHIVE-UI-001`.

## Next task

`MAINT-RELEASE-ARCHIVE-UI-001`
