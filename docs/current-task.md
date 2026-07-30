# Current task

## Active task

**ID:** TEST-RESULT-MODEL-001
**Title:** Model typed test results and comparisons

## Objective

Add pure typed state and reducer/app routes for bounded exact result identities,
correlated imports, normalized cases, comparisons, result/log opening, and
non-overwriting JUnit export.

## Required work

1. Extend `yoctui-model::testing` with bounded exact result identity, suite,
   case, outcome, metadata, log-reference, and import-state types.
2. Normalize duplicate and oversized records deterministically while preserving
   explicit partial limitations and unavailable fields.
3. Model exact baseline/current selection and deterministic comparison
   categories for regressions, new failures, fixed tests, and unchanged cases.
4. Add correlated App state, selection, import/comparison actions and effects,
   stale-result rejection, typed result/log opening, and explicit JUnit export
   destination/preview/outcome state.
5. Ensure JUnit export validation rejects existing, relative, escaping, or
   otherwise unsafe destinations before an effect is emitted.
6. Add pure unit, reducer, and app mapping tests for normal, empty, partial,
   malformed, stale, bounded, and export failure paths.

## Definition of done

- Result records use stable exact identities and bounded normalized content.
- Import and comparison states distinguish not loaded, loading, empty,
  available, partial, and failed without fabricating results.
- Comparison categories are deterministic and identity-correlated.
- Result/log opening and JUnit export are typed effects; export never overwrites
  an existing destination.
- Reducer/app tests cover bounds, stale events, partial data, selection, and
  export validation/outcomes.

## Verification

```bash
cargo test -p yoctui-model test_results
cargo test -p yoctui-app test_results
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
