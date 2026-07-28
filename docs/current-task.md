# Current task

## Active task

**ID:** TEST-MODEL-001
**Title:** Model typed test discovery and execution

## Objective

Add pure typed state and reducer/app routes for Testing capability, launch
selection and previews, stable managed sessions, lifecycle, bounded output,
cancellation, and terminal outcomes.

## Required work

1. Add a focused `yoctui-model::testing` module for bounded family, selector,
   capability, draft, request, exact preview, session, and stream types.
2. Add `Testing` to the stable Navigator after `SDK` without implementing
   result records that belong to `TEST-RESULT-MODEL-001`.
3. Reuse exact `BuildRequest` values for testimage, testsdk, testsdkext, and
   configured ptest; represent selftests as validated shell-free requests.
4. Add App state, typed actions/effects, dialog/focus behavior, stale-event
   rejection, shared background-job lifecycle, bounded output, cancellation,
   rejection, timeout, failure, and loss.
5. Add app input mapping for view-independent launch selection and every
   launch/preview/cancellation dialog key without implementing widgets.
6. Add pure unit, reducer, and app mapping tests for normal and failure paths.

## Definition of done

- Every launch family has validated typed identity and exact preview.
- Testing navigation and session lifecycle persist independently of screen
  selection.
- Build tasks reuse `BuildRequest`; no shell string represents a selftest.
- Reducer/app tests cover bounds, stale events, cancellation, and all terminal
  outcomes.

## Verification

```bash
cargo test -p yoctui-model test_workflow
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
