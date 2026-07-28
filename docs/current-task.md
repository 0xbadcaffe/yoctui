# Current task

## Active task

**ID:** SDK-001
**Title:** SDK build and artifact workflow

## Objective

Close the SDK parent completion gate by verifying all completed SDK child
tasks together against the authoritative product, UI, and architecture
contracts.

## Required work

1. Re-read every SDK requirement in `docs/ui-spec.md` and the completed SDK
   child-task evidence.
2. Run the complete SDK verification matrix and baseline checks without
   weakening or bypassing any test.
3. Reconcile only genuine remaining contract gaps; do not duplicate the
   completed cross-layer implementation.
4. Keep live-compatibility claims explicitly separate from fake
   filesystem/process evidence.
5. Mark the parent complete only when all child tasks and verification pass,
   then select the next eligible highest-priority incomplete task.

## Definition of done

- All required SDK child tasks are `DONE`.
- The combined SDK model, app, adapter, UI, and CLI verification passes.
- The baseline checks pass.
- SDK status and evidence are reconciled without claiming unperformed live
  validation.

## Verification

```bash
cargo test -p yoctui-model sdk_workflow
cargo test -p yoctui-app sdk_workflow
cargo test -p yoctui-bitbake sdk_
cargo test -p yoctui-ui sdk_workflow
cargo test -p yoctui -- sdk_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
