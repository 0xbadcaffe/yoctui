# Current task

## Active task

**ID:** DEVTOOL-JOB-LIFECYCLE-001
**Title:** Integrate Devtool persistent job lifecycle

## Objective

Map typed Devtool operations and runner events into the existing persistent
background-job reducer and CLI event loop without terminal suspension.

## Required work

1. Inventory background-job reducer transitions/retention, build coordinator,
   Devtool reducer effects/direct CLI helpers, cancellation routing, UI job
   summaries, and asynchronous loop structure.
2. Add a Devtool job coordinator that allocates stable IDs, creates
   `BackgroundJobKind::Devtool` specs with operation/recipe context, rejects a
   duplicate active operation, and maps every runner event to existing typed
   background-job actions.
3. Preserve stdout/stderr identity in retained output severity/context without
   parsing UI text; mark truncation explicitly.
4. Route start failures, nonzero exits, successful completion, acknowledged or
   forced cancellation, runner loss, and cancellation failure to exact reducer
   terminal states.
5. Start and poll the runner asynchronously while normal navigation/rendering
   continues; remove terminal suspension from migrated Devtool execution.
6. Route cancellation only to the matching active Devtool job and keep
   duplicate/late requests inert.
7. Ensure completed Devtool output and outcome remain available after screen
   navigation and do not interfere with an active BitBake job.
8. Add reducer/app/CLI tests named `devtool_job_lifecycle` for success,
   duplicate rejection, output retention, navigation, all failure/cancel/loss
   outcomes, and independent BitBake coordination.
9. Update architecture and UI specification for persistent Devtool behavior.

## Definition of done

- Devtool runner events use the existing persistent job reducer lifecycle.
- TUI navigation/rendering remains active during Devtool execution.
- Duplicate, cancellation, failure, and loss semantics are deterministic.
- Devtool and BitBake active-job coordination cannot corrupt each other.
- Focused and baseline verification pass.
- Parent Devtool-jobs task is DONE and modify/edit/build becomes active.

## Verification

```bash
cargo test -p yoctui-model devtool_job_lifecycle
cargo test -p yoctui-app devtool_job_lifecycle
cargo test -p yoctui -- devtool_job_lifecycle
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-MODIFY-001 — Complete Devtool modify, edit, and build workflow`
