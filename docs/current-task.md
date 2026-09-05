# Current Task

## Task

**ID:** PERF-IPC-LATENCY-001
**Title:** Measure saturated IPC latency
**Status:** IN_PROGRESS

## Objective

Measure daemon-event delivery, client-command receipt, and cancellation
acknowledgement latency under deterministic full-CPU saturation.

## Dependencies

- PERF-SATURATION-HARNESS-001 — DONE
- PERF-IPC-BACKPRESSURE-001 — DONE
- PERF-BITBAKE-CONN-001 — DONE

## Definition of done

- At least 100 post-warmup observations exist for daemon-event to client,
  client-command to daemon, and cancellation request to acknowledgement.
- Measurements use monotonic timestamps while every affinity CPU is runnable.
- Ordinary IPC reports p50 <=25 ms and p95 <=100 ms.
- Cancellation acknowledgement reports p95 <=250 ms.
- Evidence proves connection continuity, exact ordering, revision, host, load,
  transport, and method.

## Verification

```bash
./scripts/verify-ipc-continuity.sh --latency
./scripts/verify-roadmap.sh
```

Saturated input latency is complete in v0.1.41.
