# Current task

## Active task

**ID:** QA-001
**Title:** Recipe, kernel, and layer QA workflows

## Objective

Close the QA parent completion gate after all specification, model, adapter,
rendering, and CLI child tasks have passed.

## Required work

1. Verify every QA child task is `DONE` and its dependency is satisfied.
2. Run the complete QA parent matrix from the committed cross-layer state.
3. Run the full baseline without weakening checks or treating fixtures as
   live Yocto compatibility evidence.
4. Reconcile the registry and human-readable status only when every command
   passes.

## Definition of done

- Every QA parent verification command passes.
- `QA-001` and the M5 QA workflow are marked `DONE`.
- The next eligible highest-priority incomplete task becomes current.

## Verification

```bash
cargo test -p yoctui-model qa_
cargo test -p yoctui-app qa_workflow
cargo test -p yoctui-bitbake qa_
cargo test -p yoctui-ui qa_workflow
cargo test -p yoctui -- qa_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
