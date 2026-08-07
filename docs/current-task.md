# Current Task

## Task

**ID:** RELVAL-FLOW-001
**Title:** Verify Tab focus, windows, dialogs, and workspace flow
**Status:** NOT_STARTED

## Objective

Exercise forward/reverse focus, Navigator-to-Workspace-to-Inspector traversal,
narrow pane switching, overlays, nested modal trapping/restoration, search,
back navigation, and persistent jobs across supported terminal sizes.

## Verification

```bash
cargo test -p yoctui-e2e navigation_flow
./scripts/test-tui-flow.sh
```

## Definition of done

- Focus and workspace transitions remain deterministic across resize and modal
  boundaries, with exact return-focus restoration.

## Next task

After completion, select `RELVAL-VISUAL-001`.
