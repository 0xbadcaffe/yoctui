# Current task

## Active task

**ID:** RECIPE-NAV-001
**Title:** Complete recipe file, log, patch, and Devtool routes

## Objective

Complete authoritative navigation from the selected recipe to its provider
file, task logs, patches, and existing typed Devtool workflows.

## Required work

1. Inventory existing editor, retained task-log, patch-preview, Devtool dialog,
   effect, CLI, and Recipes Inspector behavior before adding routes.
2. Open the selected recipe's authoritative provider through the configured
   editor effect, preserving the application and surfacing missing-path,
   missing-editor, and process failures.
3. Add a typed task-log picker when more than one retained log belongs to the
   selected recipe; open the selected existing log and explain empty, evicted,
   or missing paths without guessing.
4. Add a typed patch picker/review route backed by authoritative selected
   recipe metadata. Resolve only supported local patch paths and explain
   unavailable or remote-only entries.
5. Integrate the existing typed Devtool modify, update-recipe, finish, reset,
   and deploy-target routes with the absolute selected recipe identity.
6. Show route availability and exact disabled explanations in the Recipes
   Inspector/footer. All pickers trap focus and degrade safely at every
   responsive breakpoint.
7. Cover reducer, input, editor/process failures, dialogs, Devtool routing, and
   responsive modes with tests named `recipe_navigation`.

## Definition of done

- Provider, retained task-log, and supported local patch routes use
  authoritative typed paths.
- Multiple logs or patches use focus-trapping typed pickers.
- Missing, evicted, unsupported, remote, and process-failure states are visible
  and never fabricate paths.
- Existing Devtool operations are reachable from the absolute selected recipe
  and retain their confirmation/error semantics.
- Responsive Recipes views expose route availability and disabled reasons.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model recipe_navigation
cargo test -p yoctui-app recipe_navigation
cargo test -p yoctui-ui recipe_navigation
cargo test -p yoctui -- recipe_navigation
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`RECIPE-QA-001 — Add recipe CVE and SPDX background actions`
