# Current Task

## Task

**ID:** PERF-CPU-AFFINITY-001
**Title:** Evaluate optional CPU affinity and isolation
**Status:** IN_PROGRESS

## Objective

Measure whether reserving or preferring a logical CPU materially improves
Yoctui responsiveness under saturation. Keep affinity optional and prove
correctness without a deliberately free CPU.

## Dependencies

- PERF-BASELINE-001 — DONE
- PERF-SATURATION-HARNESS-001 — DONE

## Definition of done

- Full-affinity and one-reserved-CPU scenarios use the same repeated monotonic
  latency method and record exact CPU sets.
- Any measured improvement is reported without presenting affinity as required.
- Yoctui does not hardcode a CPU number or mutate process affinity by default.
- The standard no-free-CPU saturation gate remains passing.
- Optional commands are topology-aware, unprivileged, and documented only when
  supported by evidence.

## Verification

```bash
./scripts/verify-performance.sh --affinity
./scripts/verify-roadmap.sh
```

Normal inherited scheduler behavior and optional nice/CPUWeight evaluation are
complete in v0.1.38.
