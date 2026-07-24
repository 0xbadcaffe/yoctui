# Current task

## Active task

**ID:** RECIPES-UI-001
**Title:** Complete searchable Recipes Inspector

## Objective

Render the authoritative recipe summary/detail model as a bounded searchable
Recipes workspace and contextual Inspector across all responsive modes.

## Required work

1. Inventory the existing recipe table, metadata search, selection, global
   Inspector, dependency view, footer, responsive layouts, and tests.
2. Render recipe name, preferred/resolved version, providing layer, append
   count, workspace/Devtool state, and current build state without inventing
   unavailable data.
3. Make search/filter match names, versions, layers, and provider paths while
   keeping selection stable by recipe identity across filtering and refresh.
4. Lazily request selected recipe detail and show loading, available-empty,
   unavailable, and failure states distinctly.
5. Populate contextual sections for dependencies/reverse dependencies, tasks,
   metadata sources, patches, package outputs, and history from typed state.
6. Derive current per-recipe task/build state only from typed active/completed
   tasks and build history; do not copy or parse log text.
7. Keep unavailable operations visible with explanations in the Inspector or
   footer.
8. Cover empty, large, filtered, refreshed, partial/failed metadata, active
   build, and every responsive mode with reducer, input, and `TestBackend`
   tests named `recipes_workspace`.

## Definition of done

- Rows expose every required summary field with honest unavailable values.
- Filtering and selection are stable, bounded, and deterministic.
- `Enter` refreshes only the selected recipe's typed detail.
- Inspector sections distinguish unavailable, empty, and populated values.
- Build/task state is derived only from authoritative typed state.
- Wide, medium, narrow, and too-small rendering never panic.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model recipes_workspace
cargo test -p yoctui-ui recipes_workspace
cargo test -p yoctui-app recipes_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`RECIPES-ACTIONS-001 — Complete typed recipe actions`
