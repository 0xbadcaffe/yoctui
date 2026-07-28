# Current task

## Active task

**ID:** SDK-UI-CLI-001
**Title:** Integrate complete SDK workspace

## Objective

Close the cross-layer SDK workspace gate by auditing the authoritative UI
contract against the completed model, adapters, rendering, and CLI execution,
then implement and verify any missing end-user interaction without duplicating
state or execution lifecycles.

## Required work

1. Inspect the completed SDK child tasks and every SDK requirement in
   `docs/ui-spec.md` before changing code.
2. Exercise the actual model/app/UI/CLI routes for image selection, populate
   and test previews, artifact scan/search/selection/opening, publication,
   native-tool entry and preview, and independent cancellation.
3. Implement only gaps found by that audit. Keep widgets typed, preserve the
   shared dialog/focus model, and route builds through the existing BitBake
   coordinator.
4. Add cross-layer `sdk_workflow` tests for every corrected route, including
   80x24 behavior, long inputs, navigation retention, terminal outcomes,
   child-only extracted environments, and refresh correlation.
5. Do not claim live SDK compatibility from fake filesystem/process tests.
6. Run the complete child and baseline verification, then hand off to
   `SDK-001`.

## Definition of done

- Every SDK shortcut and dialog specified in `docs/ui-spec.md` is reachable
  and usable through the real CLI input router.
- Model, app, adapters, UI, and CLI agree on exact typed identities,
  lifecycle, refresh, cancellation, and failure meaning.
- The combined SDK parent verification and baseline checks pass.

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
