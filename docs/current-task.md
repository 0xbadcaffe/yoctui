# Current Task

## Task

**ID:** UX-WIDGET-PRIMITIVES-001
**Title:** Build shared gauges meters charts tabs and scrollbar primitives
**Status:** NOT_STARTED

## Objective

Create render-only semantic primitives for the visual workbench vocabulary
without introducing widget-owned domain state or a second data authority.

## Dependencies

- `UX-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/primitives.rs`
- `crates/yoctui-ui/src/widgets.rs`
- `crates/yoctui-ui/src/theme.rs`
- typed model projections consumed by visual widgets
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Determinate gauges and meters always include exact numeric text.
- Histories, charts, bars, tabs, legends, and scrollbars use semantic roles.
- Unknown, unavailable, partial, empty, and terminal states remain explicit.
- ASCII, no-color, high-contrast, and reduced-motion fallbacks retain meaning.
- Responsive bounds and large/empty/Unicode inputs never panic or clip controls.
- Every primitive is render-only over typed bounded projections.

## Verification

```bash
cargo test -p yoctui-ui ux_widget_primitives
cargo test -p yoctui-model ux_widget_projection
./scripts/verify-roadmap.sh
```

Third-party widgets remain behind the completed license/dependency admission
gate; small native primitives are preferred when they preserve the boundary.
