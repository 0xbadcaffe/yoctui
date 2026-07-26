# Current task

## Active task

**ID:** DEVTOOL-FINISH-001
**Title:** Complete Devtool finish workflow

## Objective

Finish a clean committed Devtool workspace into an authoritative configured
layer using a typed native path, exact confirmation, persistent execution, and
original-identity refresh.

## Required work

1. Inventory finish eligibility, Git-state policy, current free-text
   destination dialog, configured layer metadata, command specification,
   persistent completion, and refresh behavior.
2. Require exact authoritative recipe identity, present workspace source, and
   clean committed Git state; preserve precise partial-state disabled reasons.
3. Replace arbitrary destination entry with a focus-trapping picker populated
   only from authoritative configured layers with absolute native paths.
4. Preview the exact `devtool finish <recipe> <destination>` intent, provider
   path, and selected configured layer before explicit confirmation.
5. Preserve native path bytes through the process specification and reject
   empty, relative, stale, or unconfigured destinations before execution.
6. Refresh the original recipe identity after successful persistent completion
   and retain job output/outcome plus prior actionable state on failure.
7. Add adapter, model, app, CLI, and Ratatui TestBackend tests named
   `devtool_publish_finish`.
8. Update UI and architecture documents for intentional behavior and boundary
   changes.

## Definition of done

- Only an authoritative clean committed workspace can begin finish.
- Destination selection cannot escape the configured layer inventory.
- Confirmation shows exact recipe, provider, layer, and native destination.
- Success refreshes the original identity; failures preserve durable context.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_publish_finish
cargo test -p yoctui-model devtool_publish_finish
cargo test -p yoctui-app devtool_publish_finish
cargo test -p yoctui-ui devtool_publish_finish
cargo test -p yoctui -- devtool_publish_finish
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-TARGET-001 — Complete Devtool deploy-target and reset workflows`
