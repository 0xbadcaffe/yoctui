# Current task

## Active task

**ID:** TEST-UI-CLI-001
**Title:** Integrate complete Testing workspace

## Objective

Close the cross-layer integration gate for the complete Testing workspace.

## Required work

1. Inspect the complete Testing implementation and focused tests before
   changing behavior.
2. Verify every specified launch family crosses model, app, adapter, renderer,
   and CLI boundaries with exact identities.
3. Verify structured result import, suite/case drill-down, comparison, JUnit
   export, cancellation, navigation, and terminal states across layers.
4. Add only missing cross-layer coverage or fixes discovered by the gate.
5. Keep live compatibility claims separate from fake process and filesystem
   coverage.

## Definition of done

- Every focused Testing verification command passes without weakening tests.
- Model, app, adapter, UI, and CLI behavior agrees with the authoritative
  specification and architecture.
- Exact launch, result, comparison, and export identities remain correlated.
- All responsive and terminal states remain explicit.
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
