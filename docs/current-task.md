# Current Task

## Task

**ID:** UX-VIEWPORT-CUES-001
**Title:** Polish long-menu viewport feedback
**Status:** DONE

## Objective

Long Navigator, menu, palette, and Compatibility collections communicate the
current selection and available scroll directions in compact title chrome.

## Dependencies

- UX-SELECTION-VIEWPORT-002 — DONE

## Definition of done

- One render-only helper derives truthful one-based selection/range cues.
- Navigator shows a compact cue only while its visible rows overflow.
- application/context menus, command palette results, and Compatibility
  capability inventory expose consistent clipped-collection cues.
- first, middle, final, fitting, Unicode, and ASCII/no-color cases are covered.
- Version 0.1.13 is installed and repository completion gates pass.

## Verification

```bash
cargo test -p yoctui-ui tests::ux_menu_renders_groups_context_disabled_safety_and_accessible_responsive_states -- --exact
cargo test -p yoctui-ui tests::ux_viewport_chrome_reports_position_and_available_directions -- --exact
cargo test -p yoctui-ui
./scripts/verify-m22-concept-parity.sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

Selection and scrolling remain model-owned. The UI helper receives only the
resolved selection, viewport, total, and presentation mode; it owns no state.

Navigator, application/context menus, command-palette results, and the
Compatibility inventory now share truthful overflow-only viewport chrome.
First, middle, final, fitting, Unicode, and ASCII/no-color states are covered
without taking a content column. Version 0.1.13 passes the required gates.
