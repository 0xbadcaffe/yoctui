# Current Task

## Task

**ID:** PERF-CI-001
**Title:** Integrate deterministic and scheduled performance CI
**Status:** IN_PROGRESS

## Objective

Add fast deterministic performance checks to pull-request CI and keep the
long-running CPU, memory, profiling, and real-Poky evidence paths scheduled or
explicitly opt-in, with retained artifacts on failure.

## Dependencies

- PERF-REGRESSION-001 — DONE

## Definition of done

- Pull requests run busy-loop, render invalidation, event coalescing, IPC
  backpressure, and deterministic saturation responsiveness checks.
- Scheduled or manually dispatched CI covers profiling, real Poky, the full
  steady-state CPU gate, and memory endurance without making PRs impractical.
- Failure artifacts preserve machine-readable performance results and logs.
- CI verification is offline and proves the required workflow coverage.

## Verification

```bash
./scripts/verify-performance.sh --ci
./scripts/verify-roadmap.sh
```

The compact regression record is complete in v0.1.48 with 22 hard metrics and
seven correctness checks regenerated from exact retained evidence.
