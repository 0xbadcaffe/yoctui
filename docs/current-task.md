# Current Task

## Task

**ID:** UX-DASHBOARD-001
**Title:** Compose a clearer operational dashboard
**Status:** NOT_STARTED

## Objective

Make the dashboard an authoritative operational overview that prioritizes the
current build, the safest next action, failures, recent work, and environment
health rather than decorative or duplicated state.

## Dependencies

- `UX-PROGRESS-001` — DONE
- `UX-THROBBER-001` — DONE
- `UX-TELEMETRY-001` — DONE

## Definition of done

- Current build and hierarchical progress are the strongest visual priority.
- Failures, recent artifacts/jobs, capability-aware next actions, telemetry,
  and environment health are reachable without duplicating source authority.
- Context navigation, pane focus, and zoom remain consistent with the shared
  action catalog and responsive layout contract.
- Wide, medium, narrow, ASCII, no-color, reduced-motion, empty, partial,
  failure, running, and completed states are deterministic and tested.

## Verification

```bash
cargo test -p yoctui-model ux_dashboard
cargo test -p yoctui-ui ux_dashboard
cargo test -p yoctui-app ux_dashboard
```
