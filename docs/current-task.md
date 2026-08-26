# Current Task

## Task

**ID:** UX-ROOTFS-UI-001
**Title:** Render rootfs pie bar table and tree exploration
**Status:** NOT_STARTED

## Objective

Turn the typed, correlated rootfs authority into a responsive Images subview
with exact package and filesystem evidence, stable drilldown, accessible chart
fallbacks, and honest lifecycle/limitation presentation.

## Dependencies

- `UX-ROOTFS-ADAPTER-001` — DONE
- `UX-LIST-TREE-001` — DONE
- `UX-WIDGET-PRIMITIVES-001` — DONE
- `UX-LICENSE-001` — DONE

## Relevant files

- Images workspace routing, catalog actions, and key hints
- rootfs tabs, package composition chart/bar/table, and filesystem tree
- package/category/path selection, scrolling, mouse rows, and Inspector detail
- narrow, ASCII, no-color, high-contrast, and screen-reader projections
- production renderer and app input tests

## Definition of done

- Installed-package and filesystem-tree authorities remain visually distinct;
  available, empty, loading, partial, unavailable, and failed states are exact.
- Wide layouts pair an admitted pie visualization with exact values; bar/table
  and text projections retain complete meaning at narrow/accessibility modes.
- `Other` remains inspectable, package/category/path drilldown is stable, and
  tree rows expose exact position, kind, bytes, ownership, and limitations.
- Keyboard/menu/mouse routes reuse typed actions and remain responsive and
  deterministic under maximum bounded model input.

## Verification

```bash
cargo test -p yoctui-ui ux_rootfs
cargo test -p yoctui-app ux_rootfs
cargo deny check
```
