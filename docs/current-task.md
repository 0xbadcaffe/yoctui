# Current task

## Active task

**ID:** DEVTOOL-RESET-001
**Title:** Complete Devtool reset workflow

## Objective

Reset an authoritative Devtool workspace through an exact destructive
confirmation and persistent cancellable job, then refresh the original identity
to its post-removal state without losing outcome context.

## Required work

1. Inventory reset eligibility, destructive confirmation, command spec,
   persistent completion/cancellation, and refresh behavior.
2. Require the exact selected `RecipeIdentity` and authoritative state that is
   either a present workspace or a reported missing workspace directory;
   missing tools, status errors, non-membership, and stale state remain inert.
3. Retain the identity through a focus-trapping destructive confirmation that
   shows the exact `devtool reset <recipe>` intent, provider path, and source
   path being removed.
4. Revalidate eligibility immediately before process construction.
5. Preserve persistent stream output, graceful/forced cancellation, nonzero
   failure, runner loss, and navigation retention.
6. Refresh only the original identity after successful completion; a
   `NotMember` result is expected, while refresh failure retains prior status
   and durable job context.
7. Add adapter, model, app, CLI, and Ratatui TestBackend tests named
   `devtool_target_reset`.
8. Update UI and architecture documents for intentional behavior and boundary
   changes.

## Definition of done

- Reset cannot start without exact authoritative removable workspace state.
- Confirmation identifies exact recipe, provider, and removal source.
- Persistent success, failure, cancellation, and loss remain distinguishable.
- Success refreshes the original identity without erasing durable context.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_target_reset
cargo test -p yoctui-model devtool_target_reset
cargo test -p yoctui-app devtool_target_reset
cargo test -p yoctui-ui devtool_target_reset
cargo test -p yoctui -- devtool_target_reset
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEP-001 — Dependency exploration and why-built workflow`
