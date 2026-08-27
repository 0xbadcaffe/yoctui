# Current Task

## Task

**ID:** UX-A11Y-001
**Title:** Verify M21 accessibility and nonvisual meaning
**Status:** NOT_STARTED

## Objective

Verify that every M21 state and interaction remains understandable without
depending on color, Unicode glyphs, animation, pointer input, or visual layout.

## Dependencies

- `UX-RESPONSIVE-001` — DONE
- `UX-THROBBER-001` — DONE
- `UX-CHECKBOX-001` — DONE

## Definition of done

- No-color, ASCII, high-contrast, and reduced-motion modes preserve every state,
  focus owner, selection, progress value, error, and disabled reason in text.
- Charts, pie slices, trees, checkboxes, gauges, meters, and throbbers expose
  exact labels/values/states without relying on hue, glyph shape, or motion.
- Menus, dialogs, terminal writer/read-only ownership, onboarding, Settings,
  logs, and compatibility states retain nonvisual meaning at all breakpoints.
- Terminal-reader projections are bounded, ordered, free of replacement glyphs,
  and distinguish empty, loading, partial, stale, failure, and terminal states.

## Verification

```bash
cargo test -p yoctui-ui ux_accessibility
cargo test -p yoctui-model ux_accessibility
./scripts/verify-ui-spec.sh
```
