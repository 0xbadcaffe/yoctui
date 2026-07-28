# Current task

## Active task

**ID:** TEST-SPEC-001
**Title:** Specify unified testing workflows

## Objective

Define the authoritative UI behavior and architecture boundaries for one
typed Testing workspace covering oe-selftest, bitbake-selftest, testimage,
testsdk, testsdkext, ptest, resulttool comparison, and JUnit export.

## Required work

1. Verify the official roles and invocation boundaries of each supported
   Yocto/BitBake testing tool.
2. Expand `docs/ui-spec.md` with exact workspace layout, typed selectors,
   shortcuts, dialogs, focus, responsive behavior, lifecycle, result
   comparison, log/metadata opening, and JUnit export.
3. Add the managed Testing boundary to `docs/architecture.md`, preserving the
   dependency direction and existing build/job coordinators.
4. Reconcile `docs/product-roadmap.md` only where the detailed contract
   requires clarification.
5. Record honest unavailable, partial, failure, cancellation, and live
   validation meaning. Never treat arbitrary shell text as the primary UX.

## Definition of done

- UI behavior is complete enough to implement without inventing interactions.
- Component ownership and execution boundaries are explicit.
- Every tool family, result state, comparison category, and export outcome has
  typed meaning.
- The registry and status documents select `TEST-MODEL-001` next.

## Verification

```bash
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
