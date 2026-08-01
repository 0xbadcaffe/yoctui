# Current Task

## Task

**ID:** MAINT-001
**Title:** Advanced maintenance workflows

## Objective

Close the Maintenance milestone parent gate by verifying that every required
atomic Maintenance task is complete and that the final cross-layer workflow
matrix and repository baseline remain green together.

## Required work

1. Audit all dependencies of `MAINT-001` in `docs/task-registry.toml`; no parent
   capability may be inferred from a partial child task.
2. Run the final model, app, adapter, Ratatui TestBackend, and CLI Maintenance
   matrix without weakening filters or tests.
3. Run the repository baseline and preserve the documented distinction between
   fixture verification and live Yocto compatibility.
4. Reconcile any failure in the smallest owning task before closing the parent.

## Definition of done

- Every required Maintenance child task is `DONE` with passing verification.
- The complete Maintenance cross-layer matrix passes.
- The repository baseline passes.
- `MAINT-001` and the M6 Maintenance status are marked `DONE`.

## Verification

```bash
cargo test -p yoctui-model maintenance_workflow
cargo test -p yoctui-app maintenance_workflow
cargo test -p yoctui-bitbake maintenance_
cargo test -p yoctui-ui maintenance_workflow
cargo test -p yoctui -- maintenance_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Mark `MAINT-001` `DONE` only after every command passes.
- Mark M6 Maintenance `DONE` in `docs/implementation-status.md`.
- Replace this file with the next eligible highest-priority task.

## Next task

`HARDEN-001`
