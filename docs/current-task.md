# Current Task

## Task

**ID:** UX-ONBOARDING-001
**Title:** Add guided first-run and workflow onboarding
**Status:** NOT_STARTED

## Objective

Guide a new operator through environment verification, target selection, first
build, logs/errors, artifacts/rootfs, and terminal use with resumable typed
steps that never execute work automatically.

## Dependencies

- `UX-WORKBENCH-CENTER-001` — DONE
- `UX-TEXTAREA-UI-001` — DONE

## Definition of done

- Onboarding steps are typed, resumable, optional, and gated by exact current
  prerequisites; viewing or resuming a guide never starts a build or process.
- Environment, target, build, diagnostic, artifact/rootfs, and terminal steps
  route into their existing authoritative workspaces and confirmation paths.
- Completed, current, blocked, skipped, stale, and unavailable steps are
  distinct in text across wide, narrow, ASCII, no-color, and reduced motion.
- Model, app, production-renderer, persistence, and CLI tests cover first run,
  resume, dismissal, capability changes, and safe completion.

## Verification

```bash
cargo test -p yoctui-model ux_onboarding
cargo test -p yoctui-app ux_onboarding
cargo test -p yoctui-ui ux_onboarding
cargo test -p yoctui -- ux_onboarding
```
