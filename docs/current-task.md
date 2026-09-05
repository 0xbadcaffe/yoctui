# Current Task

## Task

**ID:** PERF-BITBAKE-COEXIST-001
**Title:** Document BitBake coexistence guidance
**Status:** IN_PROGRESS

## Objective

Measure and document how BitBake thread counts, make parallelism, host load,
and optional cgroup policy affect coexistence. Diagnose oversubscription without
silently changing a user's Yocto configuration.

## Dependencies

- PERF-SCHED-001 — DONE
- PERF-CPU-AFFINITY-001 — DONE

## Definition of done

- `BB_NUMBER_THREADS`, `PARALLEL_MAKE`, load, and cgroup interactions are
  described accurately and tied to measured saturation evidence.
- Oversubscription detection has an explicit, read-only diagnostic method.
- Yoctui never edits `local.conf`, environment variables, or BitBake policy as
  part of performance tuning.
- Suggested values are examples for user review, not automatic defaults.
- Guidance preserves the no-root, no-reserved-CPU correctness contract.

## Verification

```bash
./scripts/verify-performance.sh --coexistence
./scripts/verify-roadmap.sh
```

Optional CPU affinity/isolation evaluation is complete in v0.1.39.
