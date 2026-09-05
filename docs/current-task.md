# Current Task

## Task

**ID:** PERF-SATURATION-HARNESS-001
**Title:** Create a deterministic CPU saturation harness
**Status:** IN_PROGRESS

## Objective

Create a deterministic, bounded, offline CPU-load fixture that can saturate a
configured set of logical CPUs without deliberately leaving one free. Make its
start, readiness, duration, shutdown, and achieved saturation machine-readable
so later responsiveness gates can run independently of a full Yocto build.

## Dependencies

- PERF-SPEC-001 — DONE

## Definition of done

- The fixture uses only local deterministic computation and requires no network,
  root privilege, real-time scheduling, or Yocto checkout.
- A caller chooses worker count or the full current affinity set; no logical CPU
  is implicitly reserved for Yoctui.
- Warmup/readiness is explicit and the workload terminates after a hard bounded
  duration, including cleanup on interruption.
- Machine-readable output records requested/available workers, monotonic timing,
  per-worker progress, and achieved host/process load.
- Tests cover validation, readiness, saturation, bounded exit, and cleanup.

## Verification

```bash
./scripts/verify-saturation-responsiveness.sh --harness
./scripts/verify-roadmap.sh
```

The pre-optimization baseline, seven workload profiles, and wakeup audit are
complete. No runtime optimization has landed.
