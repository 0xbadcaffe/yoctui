# Current task

## Active task

**ID:** ANIM-001
**Title:** Complete indeterminate task animation and reduced motion

## Objective

Finish and verify deterministic task activity animation so unknown progress is
visibly active without implying false numeric completion, with configurable
speed and a strict reduced-motion mode.

## Required work

1. Inspect the existing tick reducer, animation frame selection, task
   progress rendering, Settings controls, and current tests.
2. Keep indeterminate animation driven only by UI ticks, independent of
   backend event rate.
3. Ensure fast and slow modes advance at distinct deterministic rates.
4. Suppress motion completely when reduced motion is enabled while retaining
   an explicit nonnumeric active-state label.
5. Never show a fabricated percentage for tasks whose progress is unknown.
6. Do not animate completed, failed, or determinate task rows.
7. Verify animation remains legible in all semantic themes, no-color mode, and
   narrow supported terminals.
8. Add reducer and TestBackend coverage for tick advancement, frame cadence,
   reduced motion, unknown progress, and determinate/completed rows.
9. Update `docs/ui-spec.md` if the final reduced-motion representation needs
   clarification.

## Definition of done

- Unknown task progress uses deterministic nonnumeric activity.
- Fast and slow animation cadence is directly tested.
- Reduced motion shows a stable active label and no changing glyphs.
- Determinate and terminal task rows never use indeterminate animation.
- Theme/no-color/narrow rendering remains readable.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model animation
cargo test -p yoctui-ui animation
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`PALETTE-001 — Implement the searchable contextual command palette`
