# Current task

## Active task

**ID:** RECIPES-001
**Title:** Complete Recipes workspace actions

## Objective

Complete the searchable Recipes workspace, contextual Inspector, and typed
recipe action dialogs specified by `docs/ui-spec.md`, using authoritative
backend metadata and persistent background jobs.

## Required work

1. Inventory the existing recipe list, selection/search, build/clean/menuconfig,
   dependency, source, Devtool, dialog, backend, and test behavior.
2. Represent preferred/resolved version, provider layer, append count,
   workspace/Devtool state, build state, tasks, source paths, patches, package
   outputs, and history as typed model data; show unavailable fields honestly.
3. Keep search/filter and selection bounded across empty, refreshed, and large
   recipe sets.
4. Populate the contextual Inspector with selected recipe details,
   dependencies and reverse dependencies, tasks, sources, patches, packages,
   and history where the backend supplies them.
5. Route build, force task, clean, cleansstate, devshell, menuconfig,
   diffconfig, diffsigs, open recipe/log, Devtool lifecycle, patch review, CVE
   check, and SPDX actions through typed dialogs/effects.
6. Require explicit confirmation for destructive actions and show unsupported
   actions disabled with a concrete reason.
7. Execute long-running actions through persistent background jobs without
   blocking navigation or replacing the workbench shell.
8. Cover normal, empty, missing-metadata, unsupported-tool, failure,
   cancellation, refresh, and all responsive modes with reducer, adapter,
   input, integration, and `TestBackend` tests named for recipe actions.

## Definition of done

- Recipe rows expose the required authoritative summary without invented data.
- Search, filtering, refresh, and selection are bounded and deterministic.
- Inspector sections render available data and label unavailable data.
- Every specified operation uses typed actions/effects and appropriate dialogs.
- Destructive operations cannot execute without explicit confirmation.
- Background operations remain observable and cancellable where supported.
- Responsive rendering and unavailable external tools never panic.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-ui recipes
cargo test -p yoctui-app recipe_action
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-001 — Complete configuration provenance workspace`
