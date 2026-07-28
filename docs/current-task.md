# Current task

## Active task

**ID:** WIC-WRITE-RENDER-001
**Title:** Render protected Wic device workflow

## Objective

Render the completed typed device inventory, picker, phrase, exact command
preview, write history, and stronger cancellation warning in the Images
workspace and shared dialog layer at every supported responsive breakpoint.

## Required work

1. Inspect the completed Wic write dialog/state helpers, existing Images
   workspace/Inspector composition, Wic creation/session rendering, semantic
   palette, popup geometry, and footer compaction before editing.
2. Render write readiness or its exact disabled reason with the existing Wic
   capability section. Distinguish generated/deployed selection and display the
   exact image path and byte size used for discovery.
3. Render the focus-trapping device picker for loading, available-empty,
   available, partial, and failed inventory states. Each selectable row must
   show canonical path, major/minor, capacity, model, serial, transport,
   removable/read-only/writable flags, and mount summary without relying on
   color alone. Show every retained limitation.
4. Render bounded phrase entry with the exact required
   `WRITE <canonical-device-path>` text and inline validation. Do not imply
   that phrase entry alone starts the write.
5. Render a separate destructive command preview with the indexed exact
   shell-free argument vector plus complete image/device identity. `Enter` and
   `Esc` meanings must be explicit.
6. Extend managed Wic history/Inspector rendering for writes so exact
   image/device identity, lifecycle, output, drop/truncation counts, telemetry,
   success, nonzero failure, cancellation/incomplete warning, rejection, and
   loss remain visible across navigation.
7. Render the write-specific cancellation warning distinctly from ordinary Wic
   creation cancellation. Keep all overlays usable at 80×24 and preserve
   semantic themes, monochrome/no-color meaning, and long-content safety.
8. Add `D write device` beside existing `W`, `w`, `x`, generated-output, and
   artifact shortcuts, applying the documented compact footer behavior without
   hiding any critical action.
9. Add `wic_device_write` Ratatui `TestBackend` tests for every inventory/dialog
   and terminal state, all responsive modes, themes, long identities/content,
   exact selection styling, disabled reasons, and stronger cancellation.
10. Run focused and baseline checks, then hand off to `WIC-WRITE-CLI-001`.

## Definition of done

- All typed device inventory and write lifecycle states are explicit.
- Picker, phrase, preview, and cancellation dialogs remain usable at 80×24.
- Exact destructive identity and command are visible before confirmation.
- Footer and theme semantics make write availability and selection unambiguous.
- Focused `TestBackend` and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-ui wic_device_write
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
