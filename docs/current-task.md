# Current task

## Active task

**ID:** TEST-001
**Title:** Unified test execution and results

## Objective

Implement one unified, typed testing workflow for Yocto selftests, testimage,
testsdk, ptest, and resulttool without reducing the product to an unstructured
shell command surface.

## Required work

1. Inspect existing testing-related model, app, adapter, UI, CLI, and
   documentation behavior before writing code.
2. Reconcile the testing requirements in `docs/ui-spec.md`,
   `docs/architecture.md`, and `docs/product-roadmap.md`.
3. If the outcome is too large for one coherent commit, split `TEST-001` into
   dependency-ordered atomic child tasks and select the first child.
4. Cover pure state, reducer transitions, process/bridge integration, and
   Ratatui TestBackend behavior as applicable.
5. Keep test identities, execution lifecycle, retained results, comparison,
   export, cancellation, failure, and unavailable-tool meaning typed.
6. Do not claim live Yocto test compatibility from mocked evidence.

## Definition of done

- The task is split first when necessary to preserve coherent commits.
- All specified test families have typed, reachable workflows.
- Results, comparisons, export, lifecycle, and failures remain explicit.
- Focused and baseline verification pass without unsupported live claims.

## Verification

```bash
cargo test -p yoctui-app test_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
