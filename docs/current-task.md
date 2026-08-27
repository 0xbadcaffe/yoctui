# Current Task

## Task

**ID:** UX-001
**Title:** Complete the one-stop Yocto workbench UX milestone
**Status:** NOT_STARTED

## Objective

Run the dedicated one-stop workbench gate and unchanged full completion gate,
then close M21 only if every required child and release-quality invariant passes.

## Dependencies

- All 37 required M21 child tasks — DONE

## Definition of done

- All 38 M21 tasks are `DONE` with no waived child or evidence requirement.
- The dedicated workbench gate passes interaction, dependency, accessibility,
  performance, PTY, live-Yocto, and documentation checks.
- The unchanged strict clean-checkout completion gate passes.

## Verification

```bash
./scripts/verify-workbench-ux.sh
./scripts/verify-completion.sh
```
