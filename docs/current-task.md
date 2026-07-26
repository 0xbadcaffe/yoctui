# Current task

## Active task

**ID:** DEVTOOL-MODIFY-001
**Title:** Complete Devtool modify, edit, and build workflow

## Objective

Turn the persistent `devtool modify` operation into a capability-aware workflow
that refreshes authoritative workspace state, opens the reported source tree for
editing, and builds the workspace recipe without blocking navigation.

## Required work

1. Inventory existing Devtool availability, modify confirmation, persistent job
   completion, editor launch, metadata refresh, and recipe build behavior before
   changing code.
2. Split this task in the registry first if the verified implementation cannot
   remain one coherent commit.
3. Start modify only for an authoritative available recipe that is not already
   a workspace member; keep duplicate and disabled requests inert with an exact
   reason.
4. On successful modify completion, refresh authoritative Devtool metadata and
   use only the reported absolute workspace source path.
5. Open the configured editor on that source tree without losing persistent job
   history; preserve recoverable state when refresh or editor launch fails.
6. Allow the modified recipe to use the existing confirmed BitBake recipe-build
   workflow while Devtool output and outcomes remain visible.
7. Add model, app, CLI, and Ratatui TestBackend tests named `devtool_modify` for
   success, existing membership, disabled capability, refresh/editor failures,
   navigation retention, and workspace recipe build coordination.
8. Update UI and architecture documents for every intentional behavior or
   boundary change.

## Definition of done

- Modify eligibility is derived from authoritative typed Devtool state.
- Successful completion refreshes and uses the authoritative source path.
- Editor failures are recoverable and do not erase the completed job.
- A workspace recipe can be confirmed and built through the typed BitBake path.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model devtool_modify
cargo test -p yoctui-app devtool_modify
cargo test -p yoctui-ui devtool_modify
cargo test -p yoctui -- devtool_modify
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-PUBLISH-001 — Complete Devtool update-recipe and finish workflows`
