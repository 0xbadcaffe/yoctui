# Current Task

## Task

**ID:** UX-DASHBOARD-FOCUS-DENSITY-001
**Title:** Fix Dashboard focus, Navigator reopening, preview density, and message visibility
**Status:** DONE

## Objective

Keep passive UI out of keyboard focus, make collapsed Navigator roots reliably
reopen, allocate unused Layers tree width to the preview, and show guidance in
a visible popup.

## Dependencies

- UX-WORKBENCH-PARITY-001 — DONE

## Definition of done

- Dashboard never admits its read-only Tasks cockpit to pane focus, even when
  active, completed, or retained build data is present.
- Direct focus commands, Dashboard navigation, and modal restoration fall back
  to Navigator.
- Collapsed Navigator roots retain stable selection and reopen with Right,
  Enter, or another click.
- The Layers tree column is bounded by useful label width and the remaining
  workspace is assigned to the scrollable file preview.
- Guidance and failure messages appear in a cleared, dismissible popup without
  becoming a focus trap.
- Focused, workspace, Clippy, docs, and roadmap checks pass.

## Verification

```bash
cargo test -p yoctui-model dashboard_navigator_focus_always_skips_read_only_panes
cargo test -p yoctui-model collapsed_navigator_root_remains_selected_and_can_reopen
cargo test -p yoctui-app next_generation_navigator_mouse_and_keyboard_share_typed_routing
cargo test -p yoctui-app visible_guidance_popup_owns_its_advertised_enter_and_escape_controls
cargo test -p yoctui-ui layer_browser_gives_unused_tree_width_to_the_file_preview
cargo test -p yoctui-ui renders_notification
cargo test -p yoctui-ui
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
