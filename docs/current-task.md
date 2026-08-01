# Current Task

## Task

**ID:** MAINT-RELEASE-UI-001
**Title:** Add typed Maintenance release forms

## Objective

Expose model-owned, focus-trapped entry forms for locked-signature cache
generation, build-history comparison, and local Git archival without duplicating
adapter validation or starting a process.

## Required work

1. Map Release shortcuts `l`, `h`, and `a` only when their exact capability and
   required metadata are available; disabled routes remain inert with a visible
   typed reason.
2. Add bounded typed drafts and reducer actions for every field required by
   `LockedSignatureCacheRequest`, `BuildComparisonRequest`, and
   `GitArchiveRequest`. Keep authoritative metadata read-only and distinguish
   optional values from empty required values.
3. Validate drafts into exact typed requests and emit preview effects only on
   `Enter`; `Esc` closes without effects, dialogs trap focus, and ordinary pane
   shortcuts cannot leak through.
4. Define the exact field order, toggle controls, destructive/replacement
   meaning, and local-versus-network archive intent in `docs/ui-spec.md`, then
   render all three forms safely at 80x24 and responsive boundaries.
5. Add reducer, app-input, and Ratatui `TestBackend` tests for normal entry,
   invalid input, disabled capability, focus traversal/toggles, cancellation,
   bounded text, and narrow rendering. Do not claim live release-tool
   compatibility.

## Definition of done

- Release `l/h/a` opens only the specified typed form with authoritative
  context and visible side-effect meaning.
- Valid submission emits one exact typed preview effect and never spawns.
- Invalid or unavailable submissions remain visible and side-effect free.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model maintenance_release_workspace
cargo test -p yoctui-app maintenance_release_workspace
cargo test -p yoctui-ui maintenance_release_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` in the implementation commit with exact controls.
- Update `docs/architecture.md` only if component ownership changes.
- Mark `MAINT-RELEASE-UI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-RELEASE-CLI-001`.

## Next task

`MAINT-RELEASE-CLI-001`
