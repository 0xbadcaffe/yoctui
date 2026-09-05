# Current Task

## Task

**ID:** PERF-REGRESSION-001
**Title:** Track machine-readable performance regressions
**Status:** IN_PROGRESS

## Objective

Aggregate the retained M46 CPU, latency, wakeup, render, pressure, and memory
evidence into one compact machine-readable regression record with robust hard
release gates and explicit tolerance for tiny uncontrolled variance.

## Dependencies

- PERF-CPU-GATE-001 — DONE
- PERF-RESPONSIVENESS-GATE-001 — DONE
- PERF-IPC-GATE-001 — DONE
- PERF-MEMORY-GATE-001 — DONE

## Definition of done

- A compact record names the exact source artifacts and carries CPU, latency,
  wakeup, render, IPC-pressure, and memory metrics.
- Hard correctness and controlled release thresholds remain strict.
- Small variance in explicitly uncontrolled diagnostic measurements does not
  create a false regression failure.
- The verifier recalculates or cross-checks retained values rather than
  trusting an unbound summary.

## Verification

```bash
./scripts/verify-performance.sh --regressions
./scripts/verify-roadmap.sh
```

The supported real-Poky saturation gate is complete in v0.1.47: combined
daemon/client CPU was 0.9662% of one logical CPU at 99.6646% host utilization.
