# Current Task

## Task

**ID:** PERF-WAKEUPS-001
**Title:** Measure wakeups and timer behavior
**Status:** IN_PROGRESS

## Objective

Measure and audit every periodic wakeup source in the exact pre-optimization
runtime. Identify UI, animation, telemetry, IPC, reconnect, PTY, log, job, and
status work that still runs when no state changed, with reproducible process
statistics and code-location evidence before changing loop behavior.

## Dependencies

- PERF-BASELINE-001 — DONE

## Definition of done

- Periodic timers and polling loops are cataloged with exact intervals, source
  locations, visibility/need guards, and whether unchanged work is performed.
- Idle daemon and attached-client wakeups are measured across a robust window.
- Available context-switch, syscall, timer, and scheduler evidence is retained;
  unavailable host counters are recorded honestly rather than invented.
- The report distinguishes necessary liveness work from removable polling and
  names follow-up ownership without implementing speculative optimization.
- Compact machine-readable artifacts and an offline verifier are retained.

## Verification

```bash
./scripts/verify-performance.sh --wakeups
./scripts/verify-roadmap.sh
```

The pre-optimization baseline and seven workload profiles are complete. No
runtime optimization has landed.
