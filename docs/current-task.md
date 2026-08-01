# Current Task

## Task

**ID:** MAINT-RELEASE-ARCHIVE-UI-001
**Title:** Add typed Git archive form

## Objective

Expose a model-owned, focus-trapped `oe-git-archive` form that distinguishes
local archive creation from optional network push intent without starting a
process.

## Required work

1. Map Release shortcut `a` only when the exact Git archive capability is
   available.
2. Add a bounded typed draft for data directory, Git directory, create/bare/tag
   choices, branch/tag/message templates, comma-separated exclusions,
   `reference=/absolute/file` notes, and optional push remote.
3. Define deterministic field traversal, toggle and text editing, exact
   `GitArchiveRequest` validation, `Enter` preview, and side-effect-free `Esc`
   cancellation. Network intent must remain typed and must not imply a push.
4. Document exact controls/defaults and local-versus-network meaning in
   `docs/ui-spec.md`; render all fields, validation, creation/replacement risk,
   and deferred second push confirmation safely at 80x24 and responsive
   boundaries.
5. Add reducer, app-input, and Ratatui `TestBackend` tests for valid/invalid
   entry, unavailable capability, traversal/toggles, cancellation, bounds,
   local-versus-push intent, notes parsing, and narrow rendering.

## Definition of done

- `a` opens only the typed archive form and exposes every adapter-owned input.
- Valid submission emits one exact typed local/archive preview effect and never
  spawns or pushes.
- Invalid/unavailable requests remain visible or inert as specified.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model maintenance_release_archive_workspace
cargo test -p yoctui-app maintenance_release_archive_workspace
cargo test -p yoctui-ui maintenance_release_archive_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` in the implementation commit with exact controls.
- Update `docs/architecture.md` only if component ownership changes.
- Mark `MAINT-RELEASE-ARCHIVE-UI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-RELEASE-UI-001`.

## Next task

`MAINT-RELEASE-UI-001`
