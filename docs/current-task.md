# Current task

## Active task

**ID:** TASKS-001
**Title:** Complete the live Tasks workspace

## Objective

Turn the existing active-task list into the complete live build monitor
specified by `docs/ui-spec.md`, driven entirely by typed model state.

## Required work

1. Inventory the existing task, build, selection, filter, and inspector state
   before adding fields or actions.
2. Represent active, waiting, completed, and failed task rows in typed model
   state without parsing backend text in widgets.
3. Compute honest overall completed/total progress and summary counts.
4. Add bounded selection and preserve it safely as task rows arrive, complete,
   fail, or are evicted.
5. Implement the specified active, waiting, completed, failed, recipe, task,
   worker, and duration-threshold filters.
6. Populate the contextual Inspector from the selected task with available
   live log, metadata, recipe, PID, timing, dependency, source-log, and
   cancellation state; label unavailable values honestly.
7. Keep the workspace useful in wide, medium, narrow, idle, running, completed,
   failed, cancelled, and backend-loss states.
8. Map typed keyboard input for selection and filter interaction through
   `yoctui-app`.
9. Add reducer, input-mapping, and Ratatui `TestBackend` coverage named
   `live_tasks`.

## Definition of done

- Overall progress and task-state counts are accurate and never fabricate
  completion.
- Every specified task state and filter is visible and testable.
- Selection is bounded and drives contextual Inspector content.
- Task rows and Inspector consume typed model state only.
- Responsive and terminal build states render without panic.
- Reducer, input-mapping, and TestBackend tests cover the complete workspace.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model live_tasks
cargo test -p yoctui-ui live_tasks
cargo test -p yoctui-app live_tasks
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`LOG-001 — Complete bounded searchable logs`
