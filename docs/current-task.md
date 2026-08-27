# Current Task

## Task

**ID:** UX-RESPONSIVE-001
**Title:** Verify every M21 workflow across responsive layouts
**Status:** NOT_STARTED

## Objective

Verify every M21 workflow at all supported terminal breakpoints without hidden
controls, lost selection, focus ambiguity, overflow, or replacement glyphs.

## Dependencies

- `UX-DEPENDENCY-GRAPH-001` — DONE
- `UX-IMAGE-PREVIEW-001` — DONE
- `UX-ONBOARDING-001` — DONE
- `UX-PREFERENCES-001` — DONE

## Definition of done

- Menus, editors, dependency graphs, rootfs, terminal sessions, Dashboard,
  Workbench Center, onboarding, Settings, and dialogs remain usable at 200x60,
  160x50, 130x40, 100x30, and 80x24.
- Below-minimum terminals show a bounded recovery message rather than partial
  controls or a panic.
- Resize preserves typed selection, scroll identity, focus, modal ownership,
  and terminal ownership across wide, medium, and narrow topology changes.
- Snapshot and semantic tests reject clipping, overflow, replacement glyphs,
  inaccessible active controls, and lost state at every required breakpoint.

## Verification

```bash
cargo test -p yoctui-ui ux_responsive
cargo test -p yoctui-app ux_responsive
./scripts/test-tui-snapshots.sh
```
