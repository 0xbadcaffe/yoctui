# Current task

## Active task

**ID:** RECIPES-ACTIONS-001
**Title:** Complete typed recipe actions

## Objective

Complete the Recipes workspace's specified operations using typed actions,
capability-aware dialogs/effects, explicit destructive confirmation, and
persistent background jobs.

## Required work

1. Inventory every existing recipe build/task, editor/log, dependency,
   Devtool, process/bridge, confirmation, background-job, and test path.
2. Implement build and force-task selection, clean, cleansstate, devshell,
   menuconfig, diffconfig, and diffsigs as typed actions/effects.
3. Implement open recipe/provider, open selected task log, and patch review
   using authoritative paths with visible missing-path/tool failures.
4. Complete Devtool modify, update-recipe, finish, reset, and deploy-target
   integration without duplicating the dedicated Devtool lifecycle task.
5. Implement CVE check and SPDX generation as observable background
   operations.
6. Require preview and explicit confirmation for destructive or unusual
   operations; disabled actions remain visible with concrete capability
   explanations.
7. Keep long-running operations observable across navigation and cancellable
   where the underlying tool supports cancellation.
8. Cover normal, unsupported, malformed input, launch failure, cancellation,
   confirmation, and every responsive dialog mode with tests named
   `recipe_action`.

## Definition of done

- Every Recipes action in `docs/ui-spec.md` has a typed route.
- Destructive operations cannot execute without explicit confirmation.
- Unsupported actions are disabled/explained rather than silently ignored.
- Long-running operations use persistent job state and do not block the shell.
- Paths and task names come from authoritative typed metadata.
- Failure and cancellation outcomes remain visible and actionable.
- Task-specific and baseline verification pass.
- The parent Recipes task is closed and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model recipe_action
cargo test -p yoctui-app recipe_action
cargo test -p yoctui-ui recipe_action
cargo test -p yoctui -- recipe_action
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-001 — Complete configuration provenance workspace`
