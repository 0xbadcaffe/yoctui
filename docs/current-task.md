# Current Task

## Task

**ID:** UX-MENU-001
**Title:** Implement typed application and context menus
**Status:** NOT_STARTED

## Objective

Project the operator action catalog into focus-trapped F10 application menus and
selected-item context menus that use the existing typed action routes.

## Dependencies

- `UX-ACTION-CATALOG-001` — DONE
- `UX-KEYMAP-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/action_catalog.rs`
- new typed menu model and reducer state
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- F10 opens stable Workspace/Build/Navigate/View/Tools/Help groups.
- Arrow keys, bounded typed-prefix selection, Enter, and outward Esc are trapped.
- `a` and right click open the selected item's contextual catalog actions.
- Disabled actions remain visible with exact local/capability/safety reasons.
- Activation emits the same typed route and confirmation as palette/keybindings.
- Menus render safely at wide, medium, narrow, no-color, and reduced-motion states.

## Verification

```bash
cargo test -p yoctui-model ux_menu
cargo test -p yoctui-app ux_menu
cargo test -p yoctui-ui ux_menu
cargo test -p yoctui -- ux_menu
./scripts/verify-roadmap.sh
```

Menus are catalog projections; they cannot introduce new backend behavior or
bypass the existing compatibility and confirmation boundaries.
