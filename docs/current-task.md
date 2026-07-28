# Current task

## Active task

**ID:** SDK-RENDER-001
**Title:** Render responsive SDK workspace

## Objective

Replace the SDK placeholder with the complete typed Workspace, Inspector,
dialogs, lifecycle history, and contextual footer specified in
`docs/ui-spec.md`, without parsing paths or process output in widgets.

## Required work

1. Inspect the completed SDK model/actions/dialog state, app key mapping,
   semantic theme roles, responsive shell helpers, and the Images/QEMU/Wic
   TestBackend patterns before writing code.
2. Render SDK as a first-class Navigator destination with the exact active
   machine, distro, selected image target, and authoritative SDK deploy root.
   Explicitly render not-loaded, loading, available-empty, available, partial,
   failed, search-empty, and selected-row states.
3. Render typed artifact rows and Inspector detail for exact identity, kind,
   optional SDK/machine/host/target/publication metadata, size/mtime,
   checksum/manifest associations, scan limitations, capability state, and
   retained SDK session/background-job lifecycle and stream-tagged output.
   Missing metadata must say `unavailable`.
4. Render all existing typed SDK populate/test confirmation, publication
   destination/preview, native-tool draft/preview, and cancellation dialogs.
   Dialogs must trap focus, show indexed exact previews, wrap bounded
   paths/arguments, expose validation/disabled reasons, and remain usable at
   80x24.
5. Replace the placeholder footer with the exact contextual SDK shortcuts from
   `docs/ui-spec.md`. Preserve selection/focus/lifecycle meaning across wide,
   medium, narrow, too-small, all built-in themes, monochrome, and no-color
   modes.
6. Add focused Ratatui `TestBackend` tests named `sdk_workflow` covering every
   inventory lifecycle, search/selection, unavailable metadata, partial
   limitations, job terminal outcomes/output, all SDK dialogs, focus,
   semantic selection, themes/no-color, long paths/arguments, and responsive
   boundary sizes.
7. Update `docs/ui-spec.md` only if an intentional behavior differs from its
   current SDK contract. Update architecture only if ownership changes.
8. Run focused and baseline checks, then hand off to `SDK-CLI-001`.

## Definition of done

- SDK Workspace and Inspector render only typed model state with all lifecycle
  and unavailable states explicit.
- Every specified SDK dialog and shortcut is visible, focus-safe, and
  responsive at the supported 80x24 boundary.
- No widget classifies filenames, parses output, or owns execution state.
- Focused TestBackend and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-ui sdk_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
