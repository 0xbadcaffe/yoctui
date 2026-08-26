# Current Task

## Task

**ID:** UX-FOCUS-001
**Title:** Polish pane subfocus zoom and focus restoration
**Status:** NOT_STARTED

## Objective

Add typed pane subfocus, reversible workspace zoom, visible focus identity, and
predictable outward navigation without weakening modal or terminal ownership.

## Dependencies

- `UX-MENU-001` — DONE
- `UX-KEYMAP-MODEL-001` — DONE

## Relevant files

- typed focus/subfocus/zoom model and reducer state
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Every visible pane and subview has typed, textual focus identity.
- Zoom is reversible and restores exact focus, selection, and offsets.
- `Esc` moves outward predictably and direct menu/palette focus actions work.
- Resize preserves valid focus and never targets hidden or disabled regions.
- Dialog and terminal ownership remain trapped across keyboard and mouse input.
- Wide, medium, narrow, no-color, and reduced-motion rendering remains safe.

## Verification

```bash
cargo test -p yoctui-model ux_focus
cargo test -p yoctui-app ux_focus
cargo test -p yoctui-ui ux_focus
cargo test -p yoctui -- ux_focus
./scripts/verify-roadmap.sh
```

Focus and zoom remain client-local presentation state; they cannot create a
second backend or terminal authority.
