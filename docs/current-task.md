# Current task

## Active task

**ID:** SEC-UI-CLI-001
**Title:** Integrate complete Security workspace

## Objective

Close the cross-layer integration gate for the complete Security workspace.

## Required work

1. Inspect the complete Security implementation and focused tests before
   changing behavior.
2. Verify capability-driven current/legacy CVE and SBOM operations cross
   model, app, adapter, renderer, and CLI boundaries with exact identities.
3. Verify report import/refresh, CVE findings/mapping, SPDX document/component
   drill, exact opens, managed builds, independent mapping, cancellation,
   navigation, and terminal states across layers.
4. Add only missing cross-layer coverage or fixes discovered by the gate.
5. Keep live compatibility claims separate from fake process and filesystem
   coverage.

## Definition of done

- Every focused Security verification command passes without weakening tests.
- Model, app, adapter, UI, and CLI behavior agrees with the authoritative
  specification and architecture.
- Exact scope, operation, session, generation, finding, report, and component
  identities remain correlated.
- All responsive, partial, empty, and terminal states remain explicit.
- No fake-process test is described as live Yocto compatibility.

## Verification

```bash
cargo test -p yoctui-model security_workflow
cargo test -p yoctui-app security_workflow
cargo test -p yoctui-bitbake security
cargo test -p yoctui-ui security_workflow
cargo test -p yoctui -- security_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
