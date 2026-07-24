# Current task

## Active task

**ID:** RECIPE-BITBAKE-001
**Title:** Complete typed recipe BitBake operations

## Objective

Complete build and standard BitBake task operations for the selected recipe
using typed task selection, confirmation, and persistent background execution.

## Required work

1. Inventory existing build, clean, cleansstate, menuconfig, build-target
   dialogs, `BuildRequest`, coordinator/job lifecycle, CLI routes, and tests.
2. Define one typed recipe-task operation model for default build, arbitrary
   force task, clean, cleansstate, devshell, menuconfig, diffconfig, and
   diffsigs.
3. Populate task choices from authoritative selected-recipe metadata and keep
   known standard tasks visible only when supported or clearly explained.
4. Add a task picker/confirmation workflow that previews the exact
   `bitbake <recipe> -c <task>` intent without exposing an unstructured shell
   command.
5. Require explicit confirmation for clean, cleansstate, forced, or otherwise
   unusual task execution.
6. Start confirmed operations through the existing persistent build-job
   coordinator so navigation, output, cancellation, and terminal outcomes
   remain observable.
7. Reject empty/stale selections, unsupported tasks, duplicate active jobs,
   and malformed task names with actionable messages.
8. Cover reducer, input, dialogs, coordinator/CLI routing, failure,
   cancellation, and responsive modes with tests named
   `recipe_bitbake_action`.

## Definition of done

- Every standard Recipes BitBake operation has a typed route.
- Task choices use authoritative task metadata and never interpolate raw input.
- Exact target/task intent is previewed before unusual/destructive execution.
- Confirmed work uses persistent job state and remains navigable/cancellable.
- Invalid, unsupported, duplicate, failed, and cancelled paths are visible.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model recipe_bitbake_action
cargo test -p yoctui-app recipe_bitbake_action
cargo test -p yoctui-ui recipe_bitbake_action
cargo test -p yoctui -- recipe_bitbake_action
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`RECIPE-NAV-001 — Complete recipe file, log, patch, and Devtool routes`
