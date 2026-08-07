# Current Task

## Task

**ID:** RELVAL-VISUAL-001
**Title:** Add semantic and visual terminal regression tests
**Status:** NOT_STARTED

## Objective

Capture normalized terminal-cell snapshots for workspaces, dialogs, lifecycle
states, themes, no-color, reduced motion, Unicode/path content, and supported
terminal breakpoints.

## Verification

```bash
./scripts/test-tui-snapshots.sh
```

## Definition of done

- Semantic and visual snapshots are deterministic, bounded, and retain failure
  diffs and rendered artifacts.

## Next task

After completion, select `RELVAL-POKY-001`.
