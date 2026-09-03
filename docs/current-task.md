# Current Task

## Task

**ID:** UX-LAYER-KEY-ROUTING-001
**Title:** Route Enter and horizontal hierarchy keys to Layers and Navigator
**Status:** DONE

## Objective

Layers and Navigator receive hierarchy activation and horizontal movement
before generic pane-focus or passive-notification routing.

## Dependencies

- UX-LAYER-TREE-WIDGET-001 — DONE

## Definition of done

- Configured Layers opens with `Enter`, `Right`, or `l` even when a passive
  notification is visible.
- The open tree owns `Right`/`l`, `Left`/`h`, and `Enter` hierarchy behavior.
- Navigator arrows expand/collapse groups; `Tab`/`Shift+Tab` change panes.
- Focused app, CLI, and reducer regressions cover each route.
- Workspace, Clippy, documentation, roadmap, and completion checks pass.

## Verification

```bash
cargo test -p yoctui-app layers_workspace_owns_horizontal_hierarchy_keys_before_pane_focus
cargo test -p yoctui-app navigator_arrows_expand_and_collapse_groups_without_changing_panes
cargo test -p yoctui layers_list_enter_right_and_l_open_the_selected_hierarchy
cargo test -p yoctui focus_routing_notifications_consume_only_their_documented_keys
cargo test -p yoctui-model layer_tree_loads_children_lazily_and_collapses_without_losing_parent
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

Tab remains pane navigation; hierarchy arrows belong to the focused hierarchy.
