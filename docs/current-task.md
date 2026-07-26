# Current task

## Active task

**ID:** DEVTOOL-PUBLISH-001
**Title:** Complete Devtool update-recipe and finish workflows

## Objective

Provide capability-aware, explicitly previewed update-recipe and finish
workflows whose persistent completion refreshes authoritative Devtool state
without losing output or failure context.

## Required work

1. Inventory the existing status eligibility, dialogs, command specs,
   persistent completion, layer destination selection, Git-state policy, and
   refresh behavior before changing code.
2. Split this task in the registry first if update-recipe and finish cannot be
   implemented and verified as one coherent commit.
3. Require the exact authoritative recipe identity and workspace state for both
   operations; retain precise disabled reasons for missing tools, status errors,
   absent sources, dirty/conflicted Git state, and invalid destinations.
4. Preview the exact shell-free update-recipe or finish intent and require
   explicit confirmation.
5. Validate finish destinations against authoritative configured layer paths;
   never accept a guessed or arbitrary raw path.
6. Preserve persistent stdout/stderr and terminal outcomes, refresh the
   original recipe identity after success, and retain recoverable state after
   refresh or command failure.
7. Add model, app, CLI, adapter, and Ratatui TestBackend tests named
   `devtool_publish` for eligibility, previews, destination validation,
   success/refresh, nonzero failure, navigation retention, and partial states.
8. Update UI and architecture documents for every intentional behavior or
   boundary change.

## Definition of done

- Update-recipe and finish use authoritative typed eligibility.
- Every execution follows an exact preview and explicit confirmation.
- Finish destinations are configured layers and remain native absolute paths.
- Successful completion refreshes the original identity; failures retain job
  output, outcome, and actionable context.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_publish
cargo test -p yoctui-model devtool_publish
cargo test -p yoctui-app devtool_publish
cargo test -p yoctui-ui devtool_publish
cargo test -p yoctui -- devtool_publish
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-TARGET-001 — Complete Devtool deploy-target and reset workflows`
