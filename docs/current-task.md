# Current task

## Active task

**ID:** DEVTOOL-UPDATE-001
**Title:** Complete Devtool update-recipe workflow

## Objective

Require authoritative eligibility and an exact preview for update-recipe, then
refresh the original recipe identity after successful persistent completion
without losing retained output or failure context.

## Required work

1. Inventory existing update-recipe availability, dialog state, command
   specification, persistent completion, and status refresh behavior.
2. Require the exact selected `RecipeIdentity` to have authoritative available,
   present workspace state; missing or stale state remains inert with a precise
   reason.
3. Make the focus-trapping confirmation retain the identity and preview the
   exact `devtool update-recipe <recipe>` operation plus provider path.
4. Preserve the original identity while the persistent job runs and refresh
   only that identity after successful completion, independent of navigation.
5. Keep nonzero, cancellation, runner-loss, and refresh-failure output/outcomes
   retained and actionable.
6. Add adapter, model, app, CLI, and Ratatui TestBackend tests named
   `devtool_publish_update`.
7. Update UI and architecture documents for intentional behavior and boundary
   changes.

## Definition of done

- Unknown, unavailable, and non-workspace recipes cannot start update-recipe.
- The confirmation shows exact typed intent and provider identity.
- Success refreshes the original identity without erasing the persistent job.
- Failures preserve typed status, output, and actionable context.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_publish_update
cargo test -p yoctui-model devtool_publish_update
cargo test -p yoctui-app devtool_publish_update
cargo test -p yoctui-ui devtool_publish_update
cargo test -p yoctui -- devtool_publish_update
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-FINISH-001 — Complete Devtool finish workflow`
