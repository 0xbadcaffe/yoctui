# Current Task

## Task

**ID:** UX-SCROLL-001
**Title:** Standardize scrolling across every workspace
**Status:** NOT_STARTED

## Objective

Give every bounded collection and document one predictable typed scrolling
model without replacing stable selections, search state, or follow authority.

## Dependencies

- `UX-KEYMAP-MODEL-001` — DONE

## Relevant files

- shared typed scroll model and reducer helpers
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- workspace-specific model and rendering modules
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Row, page, top/bottom, `gg`/`G`, and horizontal scrolling are consistent.
- Mouse wheel, follow/pause, search jumps, and correlated jumps share bounds.
- Resize, filtering, inventory replacement, and retention eviction clamp safely.
- Stable selected identity is retained whenever the authoritative row remains.
- Every scrollable view exposes textual current/total or retained-range state.
- Property coverage proves offsets and selections cannot escape bounded state.

## Verification

```bash
cargo test -p yoctui-model ux_scroll
cargo test -p yoctui-app ux_scroll
cargo test -p yoctui-ui ux_scroll
cargo test -p yoctui -- ux_scroll
./scripts/verify-roadmap.sh
```

Scrolling is presentation state over typed bounded collections; it cannot
create a second inventory, log-retention, or backend authority.
