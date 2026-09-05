# Current Task

## Task

**ID:** PERF-TASKS-001
**Title:** Optimize high-rate task updates
**Status:** IN_PROGRESS

## Objective

Coalesce progress by stable task identity, batch reducer work, and avoid
rebuilding or sorting complete task tables per event. Preserve every task
transition, terminal outcome, and failure exactly.

## Dependencies

- PERF-EVENT-FLOOD-001 — DONE
- PERF-RENDER-001 — DONE

## Definition of done

- Repeated progress for one stable task identity coalesces to the newest value.
- Task events reduce in bounded batches without reordering starts,
  completions, failures, cancellation, or terminal outcomes.
- Task selection/filter projections avoid per-event full sorting and avoid
  rebuilding an unchanged table every frame.
- Active and retained task collections remain bounded under event floods.
- Focused tests and `verify-performance.sh --tasks` cover identity coalescing,
  batch ordering, terminal/failure preservation, projection reuse, and bounds.

## Verification

```bash
./scripts/verify-performance.sh --tasks
./scripts/verify-roadmap.sh
```

Bounded batched log ingestion and single-pass viewport rendering are complete
in v0.1.32.
