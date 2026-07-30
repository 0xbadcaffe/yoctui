# Current task

## Active task

**ID:** TEST-001
**Title:** Unified test execution and results

## Objective

Close the unified Testing parent gate after all atomic implementation and
cross-layer tasks have passed.

## Required work

1. Confirm every dependency of `TEST-001` is `DONE`.
2. Re-run the unified Testing verification commands.
3. Reconcile the machine-readable registry and human-readable status.
4. Select the next eligible highest-priority incomplete task.

## Definition of done

- Every Testing child task and integration gate is `DONE`.
- Every unified Testing verification command passes without weakening tests.
- Registry and status report the Testing parent capability consistently.
- No fake-process test is described as live Yocto compatibility.

## Verification

```bash
cargo test -p yoctui-model test_workflow
cargo test -p yoctui-app test_workflow
cargo test -p yoctui-bitbake test_
cargo test -p yoctui-ui test_workflow
cargo test -p yoctui -- test_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
