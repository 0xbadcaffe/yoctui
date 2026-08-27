# Current Task

## Task

**ID:** UX-WORKBENCH-CENTER-001
**Title:** Create a one-stop workbench command center
**Status:** NOT_STARTED

## Objective

Make the workbench a single operational command center that unifies recent
contexts, active work, failures, artifacts, favorite commands, terminals, and
capability-aware next actions without introducing a second source of truth.

## Dependencies

- `UX-MENU-001` — DONE
- `UX-DASHBOARD-001` — DONE
- `UX-LOGS-001` — DONE
- `UX-ROOTFS-UI-001` — DONE
- `UX-TERMINAL-UX-001` — DONE

## Definition of done

- Recent contexts, active work, failures, artifacts, favorites, and terminals
  are reachable together through bounded projections of their owning models.
- Recommended actions preserve exact local/capability availability, safety,
  confirmation, and typed workflow routing.
- Keyboard, menu, focus, zoom, mouse, and responsive behavior remain coherent
  across wide, medium, narrow, ASCII, no-color, and reduced-motion layouts.
- Empty, partial, stale, disconnected, active, failed, and completed states are
  deterministic and covered in model, app, and production-renderer tests.

## Verification

```bash
cargo test -p yoctui-model ux_command_center
cargo test -p yoctui-app ux_command_center
cargo test -p yoctui-ui ux_command_center
```
