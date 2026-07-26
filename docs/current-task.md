# Current task

## Active task

**ID:** DEVTOOL-TARGET-001
**Title:** Complete Devtool deploy-target and reset workflows

## Objective

Complete capability-aware persistent deploy-target and reset operations with
validated intent, exact confirmations, deterministic cancellation/outcomes,
and authoritative status refresh.

## Required work

1. Inventory deploy/reset eligibility, target validation, dialogs, command
   specs, persistent completion, cancellation, and refresh behavior.
2. Split this task in the registry first if deploy-target and reset cannot be
   implemented and verified as one coherent commit.
3. Require exact authoritative recipe identity and operation-specific workspace
   states, preserving precise disabled reasons.
4. Validate deploy targets as one non-option value, preview the exact
   deploy-target or reset intent, and require explicit confirmation.
5. Preserve persistent stream output, cancellation mode, terminal outcomes,
   and actionable context for both operations.
6. Refresh the original identity after successful reset and after target
   operations where authoritative workspace state may change.
7. Add adapter, model, app, CLI, and Ratatui TestBackend tests named
   `devtool_target`.
8. Update UI and architecture documents for intentional behavior and boundary
   changes.

## Definition of done

- Deploy-target and reset use exact authoritative eligibility and previews.
- Invalid target and stale status requests remain inert.
- Persistent success, failure, cancellation, and loss remain distinguishable.
- Success refreshes the original identity without erasing durable job context.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_target
cargo test -p yoctui-model devtool_target
cargo test -p yoctui-app devtool_target
cargo test -p yoctui-ui devtool_target
cargo test -p yoctui -- devtool_target
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEP-001 — Dependency exploration and why-built workflow`
