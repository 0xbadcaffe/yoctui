# Current Task

## Task

**ID:** UX-PREFERENCES-001
**Title:** Unify usability and visual preferences
**Status:** NOT_STARTED

## Objective

Expose one coherent, discoverable preference surface for keybindings, visual
density, accessibility, input, panes, charts, logs, image choices, and the
terminal prefix without creating competing configuration authorities.

## Dependencies

- `UX-KEYMAP-UI-001` — DONE
- `UX-CHECKBOX-001` — DONE
- `UX-MENU-001` — DONE

## Definition of done

- Settings exposes keybindings, theme, density, Unicode/ASCII, motion, mouse,
  footer, wrap/follow, pane sizing, chart/image choices, and terminal prefix
  through typed rows with exact current values and disabled reasons.
- Preview and reset are explicit, bounded, reversible, and cannot bypass
  existing keymap, focus, terminal-prefix, accessibility, or capability rules.
- One versioned preference schema migrates legacy session fields and persists
  atomically without rewriting project or system configuration.
- Wide, compact, no-color, ASCII, reduced-motion, mouse-disabled, invalid,
  reset, persistence-failure, and restart restoration states are tested.

## Verification

```bash
cargo test -p yoctui-model ux_preferences
cargo test -p yoctui-ui ux_preferences
cargo test -p yoctui -- ux_preferences
```
