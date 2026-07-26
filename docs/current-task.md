# Current task

## Active task

**ID:** DEVTOOL-001
**Title:** Complete Devtool lifecycle

## Objective

Complete the Devtool modify/edit/build/update-recipe/finish/deploy/reset/status
and Git lifecycle as typed, persistent, failure-safe workflows.

## Required work

1. Inventory all existing Devtool reducer actions, dialogs, CLI process
   functions, source editor, workspace paths, refresh behavior, job state, and
   tests before writing code.
2. Reconcile the authoritative UI/architecture requirements with the current
   synchronous CLI implementation. If the lifecycle is not atomic, split
   `DEVTOOL-001` into dependency-ordered child tasks and commit the split
   before implementation.
3. Provide typed modify, edit, build, update-recipe, finish, deploy-target, and
   reset operations for the absolute selected recipe.
4. Add authoritative workspace status and Git state, with explicit states for
   missing Devtool, missing workspace, dirty/untracked files, conflicts, and
   unsupported operations.
5. Run long operations as persistent cancellable background jobs where the
   underlying tool permits it; retain bounded output and terminal outcomes
   while navigation remains available.
6. Require exact preview and explicit confirmation for destructive/export/
   deploy operations, validate destinations and targets, and refresh workspace
   plus recipe metadata only after success.
7. Cover normal, duplicate, unavailable, failure, cancellation, partial
   workspace, Git, dialog-focus, CLI/fake-process, and responsive UI paths.

## Definition of done

- Every required Devtool lifecycle operation has a typed route and honest
  capability state.
- Long work persists across navigation with retained output and outcomes.
- Destructive/export/deploy actions preview exact intent and require
  confirmation.
- Workspace and Git status are authoritative or explicitly unavailable.
- Success refreshes authoritative state; failure preserves recoverable work.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-app devtool
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEP-001 — Dependency exploration and why-built workflow`
