# Current Task

## Task

**ID:** UX-LAYER-TREE-WIDGET-001
**Title:** Render Layers with tui-tree-widget
**Status:** DONE

## Objective

Layers renders its bounded lazy filesystem projection through the admitted
`tui-tree-widget` crate without transferring selection, expansion, filtering,
scrolling, filesystem, or input authority out of Yoctui's typed model.

## Dependencies

- UX-LIST-TREE-001 — DONE
- UX-SELECTION-VIEWPORT-002 — DONE

## Definition of done

- Nested visible entries render with `tui-tree-widget` 0.24.1 while collapsed
  unloaded directories retain an explicit disclosure marker.
- `LayerBrowser` remains the sole selection, expansion, lazy-loading, Git,
  filter, bound, and viewport authority.
- The UI creates and discards `TreeItem`/`TreeState` values per draw and never
  routes input through widget helpers.
- Unicode, ASCII/no-color, nested selection, malformed identity, and final-row
  visibility are covered by deterministic tests.
- Dependency admission, notices, SBOM, Cargo-deny, focused UI, workspace,
  Clippy, documentation, roadmap, and completion checks pass.

## Verification

```bash
cargo test -p yoctui-ui ux_layer_browser
./scripts/verify-third-party-notices.sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

The widget is a renderer, not a filesystem browser or interaction authority.
