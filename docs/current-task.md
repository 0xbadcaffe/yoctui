# Current Task

## Task

**ID:** PERF-TOKIO-001
**Title:** Audit Tokio runtime scheduling
**Status:** IN_PROGRESS

## Objective

Measure Tokio worker/thread use, blocking work, spawned tasks, channel choices,
and timer churn. Move only measured blocking work off reactor workers and avoid
increasing runtime threads without evidence.

## Dependencies

- PERF-FLAMEGRAPH-001 — DONE
- PERF-WAKEUPS-001 — DONE

## Definition of done

- Runtime worker and blocking-pool configuration is measured and documented.
- Filesystem, child-process, and CPU-heavy work does not block async reactor
  workers where measured evidence requires isolation.
- Channel bounds and long-lived task ownership remain explicit.
- Avoidable timer churn and unnecessary spawned tasks are removed.
- Deterministic tests prove the reactor remains responsive under blocking work
  and full-CPU contention without blindly increasing worker count.

## Verification

```bash
./scripts/verify-performance.sh --tokio
./scripts/verify-roadmap.sh
```

BitBake delayed-event continuity, real EOF detection, cancellation priority,
and terminal-before-cleanup ordering are complete in v0.1.36.
