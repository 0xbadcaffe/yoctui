# Current Task

## Task

**ID:** UX-PROGRESS-001
**Title:** Implement hierarchical build task and job progress
**Status:** NOT_STARTED

## Objective

Separate and present authoritative progress for builds, parse/runqueue phases,
selected tasks, background jobs, resources, and sstate without fabricating a
percentage when the model has no valid total.

## Dependencies

- `UX-WIDGET-PRIMITIVES-001` — DONE

## Relevant files

- typed build, task, and background-job model projections
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Build, parse/runqueue, selected-task, background-job, resource, and sstate
  progress remain distinct typed projections.
- Determinate values retain exact numeric or numerator/denominator text.
- Unknown totals remain explicit and never render as zero percent.
- Estimated values are labeled as estimates.
- Terminal states freeze their last authoritative progress.
- Responsive UI, model, and app tests cover normal, unknown, partial, and
  terminal transitions.

## Verification

```bash
cargo test -p yoctui-model ux_progress
cargo test -p yoctui-ui ux_progress
cargo test -p yoctui-app ux_progress
./scripts/verify-roadmap.sh
```
