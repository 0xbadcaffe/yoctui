# Current task

## Active task

**ID:** WIC-001
**Title:** Wic image workflow

## Objective

Close the Wic parent completion gate by verifying all completed cooked-mode
creation and protected device-writing child tasks together against the
authoritative product, UI, and architecture contracts.

## Required work

1. Re-read every Wic requirement in `docs/ui-spec.md` and the completed Wic
   child-task evidence.
2. Run the complete Wic verification matrix and baseline checks without
   weakening or bypassing any test.
3. Reconcile only genuine remaining contract gaps; do not duplicate the
   completed cross-layer implementation.
4. Keep live Wic and removable-media safety claims explicitly separate from
   fake filesystem, process, and device evidence.
5. Mark the parent complete only when all child tasks and verification pass,
   then select the next eligible highest-priority incomplete task.

## Definition of done

- All required Wic child tasks are `DONE`.
- The combined Wic model, app, adapter, UI, and CLI verification passes.
- The baseline checks pass.
- Wic status and evidence are reconciled without claiming unperformed live
  validation or hardware safety.

## Verification

```bash
cargo test -p yoctui-model wic
cargo test -p yoctui-bitbake wic
cargo test -p yoctui-app wic
cargo test -p yoctui-ui wic
cargo test -p yoctui -- wic
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
