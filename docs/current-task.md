# Current task

## Active task

**ID:** DEVTOOL-JOBS-001
**Title:** Run Devtool operations as persistent background jobs

## Objective

Replace terminal-suspending Devtool execution with typed, cancellable,
persistent background jobs whose output and terminal outcomes survive
navigation.

## Required work

1. Inventory the existing background-job model/coordinator, Devtool status and
   direct process helpers, typed effects, cancellation, bounded output, and
   tests before writing code.
2. Split this task into coherent child tasks in the registry if the inventory
   confirms argument construction/process execution and lifecycle integration
   cannot remain one small commit.
3. Define typed Devtool operation/argument construction without shell command
   strings or UI text parsing.
4. Execute Devtool without suspending the TUI and retain bounded stdout/stderr,
   running state, exit status, and actionable errors.
5. Reject duplicate active operations and cover missing executable, nonzero
   exit, cancellation acknowledgement/escalation, and backend/process loss.
6. Reuse the persistent background-job lifecycle and cancellation semantics
   rather than creating a parallel untyped runner.
7. Add adapter, reducer, app, and CLI tests named `devtool_job`.
8. Update architecture/specification/status documents with intentional
   behavior changes.

## Definition of done

- Devtool argument construction is typed and shell-free.
- Operations run in the background without terminal suspension.
- Output and terminal state remain visible after navigation.
- Duplicate, cancellation, failure, missing-tool, and loss states are tested.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_job
cargo test -p yoctui-model devtool_job
cargo test -p yoctui-app devtool_job
cargo test -p yoctui -- devtool_job
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-MODIFY-001 — Complete Devtool modify, edit, and build workflow`
