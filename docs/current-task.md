# Current Task

## Task

**ID:** PERF-CPU-GATE-001
**Title:** Implement the one-percent steady-state CPU gate
**Status:** IN_PROGRESS

## Objective

Implement the release-profile steady-state CPU gate for the documented daemon
plus one attached interactive client baseline.

## Dependencies

- PERF-ANIM-001 — DONE
- PERF-TELEMETRY-001 — DONE
- PERF-LOG-001 — DONE
- PERF-TASKS-001 — DONE
- PERF-TOKIO-001 — DONE

## Definition of done

- The documented idle daemon plus one real attached 160x50 client scenario runs
  from a release build of the measured source.
- Startup and a 10-second warmup are excluded from 60 one-second steady-state
  samples.
- Daemon and client `/proc` CPU deltas are independently calculated as percent
  of one logical CPU and summed.
- The 10% trimmed-mean combined result is <=1.00%; idle daemon is <=0.20% and
  idle attached client is <=0.50%.
- Exact host, binary, PID continuity, raw samples, and method evidence is
  retained and verified offline.

## Verification

```bash
./scripts/verify-low-overhead.sh
./scripts/verify-roadmap.sh
```

Saturated IPC latency is complete in v0.1.42.
