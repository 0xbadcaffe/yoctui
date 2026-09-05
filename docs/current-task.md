# Current Task

## Task

**ID:** PERF-DOC-001
**Title:** Document low-overhead architecture and tuning
**Status:** IN_PROGRESS

## Objective

Complete the operator and developer documentation for expected CPU use,
event-driven rendering, telemetry rates, backpressure, saturated-host behavior,
optional host guidance, profiling, and every reproduction command.

## Dependencies

- PERF-BITBAKE-COEXIST-001 — DONE
- PERF-REGRESSION-001 — DONE
- PERF-REAL-POKY-001 — DONE

## Definition of done

- Expected CPU consumption and exact accounting are easy to find.
- Render, animation, telemetry, IPC/backpressure, and saturated backend behavior
  match the implemented architecture.
- Optional nice/cgroup/affinity and BitBake parallelism guidance remains safe,
  unprivileged, advisory, and never automatic.
- Profiling and all deterministic/live evidence commands are reproducible.

## Verification

```bash
./scripts/check-docs.sh
./scripts/verify-performance.sh --docs
./scripts/verify-roadmap.sh
```

Performance CI is complete in v0.1.49: fast PR checks, weekly/manual endurance,
and opt-in self-hosted real-Poky capture retain failure evidence separately.
