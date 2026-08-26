# Current Task

## Task

**ID:** UX-LOGS-001
**Title:** Polish the bounded BitBake log explorer
**Status:** NOT_STARTED

## Objective

Make retained BitBake logs fast and predictable through virtualized ranges,
consistent navigation, explicit follow state, typed filtering, bookmarks,
correlated jumps, wrapping controls, loss accounting, and bounded copy/export.

## Dependencies

- `UX-SCROLL-001` — DONE
- `UX-WIDGET-PRIMITIVES-001` — DONE

## Relevant files

- typed retained BitBake log state and bounded scroll projections
- app keyboard/mouse log actions
- Logs workspace, search/filter/follow renderers, and Inspector details
- bounded export and clipboard effects
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Rendering consumes only the visible retained range and remains bounded.
- Follow/pause, retained position, search match, filters, bookmarks, and loss
  accounting remain explicit and reducer-owned.
- Correlated task/error jumps preserve stable selection when retained.
- Wrapped and unwrapped views clamp vertical and horizontal offsets safely.
- Copy/export is bounded and reports truncation or unavailable authority.
- Empty, filtered-empty, evicted, large, Unicode, narrow, and no-color cases
  never panic or misrepresent missing records.

## Verification

```bash
cargo test -p yoctui-model ux_logs
cargo test -p yoctui-app ux_logs
cargo test -p yoctui-ui ux_logs
./scripts/verify-roadmap.sh
```
