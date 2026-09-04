# Current Task

## Task

**ID:** UX-WORKBENCH-PARITY-001
**Title:** Unify Dashboard, integrated previews, and edit-to-build behavior
**Status:** DONE

## Objective

Make startup, Dashboard, layer/recipe previews, Rootfs composition, and the
Vim-to-recipe-build loop follow the reviewed workbench interaction model.

## Dependencies

- UX-LAYER-KEY-ROUTING-001 — DONE
- UX-ROOTFS-PIE-BRAILLE-001 — DONE

## Definition of done

- Daemon startup and indeterminate work use Braille Eight Double.
- Interactive startup focuses Overview/Dashboard in Navigator and skips
  passive focus targets.
- Dashboard uses the task cockpit with honest idle/unknown progress.
- Layers and Recipes integrate scrollable previews; Layers supports paging.
- Vim-saved source retains its diff and Ctrl+B builds the owning recipe.
- Rootfs composition prioritizes the Braille `tui-piechart` visualization.
- Exact goldens, workspace tests, Clippy, docs, roadmap, and completion pass.

## Verification

```bash
cargo test -p yoctui-model external_recipe_edit_keeps_diff_and_allows_ctrl_b_build
cargo test -p yoctui-app layers_workspace_owns_horizontal_hierarchy_keys_before_pane_focus
cargo test -p yoctui-ui
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```
