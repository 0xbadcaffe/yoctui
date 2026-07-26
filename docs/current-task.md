# Current task

## Active task

**ID:** DEVTOOL-DEPLOY-001
**Title:** Complete Devtool deploy-target workflow

## Objective

Deploy an authoritative Devtool workspace recipe to one validated target
through an exact confirmation and persistent cancellable job, then refresh the
original identity without losing outcome context.

## Required work

1. Inventory deploy-target eligibility, free-text target dialog, validation,
   command spec, persistent completion/cancellation, and refresh behavior.
2. Require the exact selected `RecipeIdentity` to have authoritative available,
   present workspace state; missing or stale state remains inert with a precise
   reason.
3. Retain the identity through target entry and confirmation; validate one
   non-option target value without whitespace or control characters.
4. Show the exact `devtool deploy-target <recipe> <target>` intent and provider
   path before explicit confirmation.
5. Preserve persistent stream output, graceful/forced cancellation, nonzero
   failure, runner loss, and navigation retention.
6. Refresh only the original recipe identity after successful completion and
   retain prior status/job context on failure.
7. Add adapter, model, app, CLI, and Ratatui TestBackend tests named
   `devtool_target_deploy`.
8. Update UI and architecture documents for intentional behavior and boundary
   changes.

## Definition of done

- Deploy-target cannot start without exact authoritative workspace eligibility.
- Invalid targets never reach process construction.
- Confirmation shows exact recipe, target, and provider identity.
- Success refreshes the original identity; failures retain durable context.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_target_deploy
cargo test -p yoctui-model devtool_target_deploy
cargo test -p yoctui-app devtool_target_deploy
cargo test -p yoctui-ui devtool_target_deploy
cargo test -p yoctui -- devtool_target_deploy
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-RESET-001 — Complete Devtool reset workflow`
