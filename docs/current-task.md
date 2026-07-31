# Current task

## Active task

**ID:** QA-UI-CLI-001
**Title:** Integrate complete QA workspace

## Objective

Close the cross-layer QA gate by proving that the typed model, capability and
report adapters, native layer runner, application mappings, responsive
renderer, and non-blocking CLI coordinator agree on every required workflow
and terminal outcome.

## Required work

1. Inspect the completed QA model, adapters, application mappings, renderer,
   and CLI integration before changing code.
2. Run the complete focused QA matrix across every crate.
3. Fix only cross-layer inconsistencies exposed by that matrix without
   weakening existing tests or inventing new UI behavior.
4. Verify recipe/kernel and configured-layer capability, managed/native
   execution, independent cancellation, bounded reports, exact opens,
   navigation, responsive rendering, and terminal outcomes together.
5. Keep fake process/filesystem evidence explicitly separate from live Yocto
   compatibility claims.

## Definition of done

- Every QA child gate passes together from the committed workspace.
- Model, adapter, app, UI, and CLI identities and lifecycle states agree.
- Baseline verification passes.

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
