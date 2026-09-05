# Current Task

## Task

**ID:** PERF-IPC-BACKPRESSURE-001
**Title:** Implement bounded priority-aware IPC backpressure
**Status:** IN_PROGRESS

## Objective

Isolate every slow client from daemon and BitBake ingestion with bounded
outbound queues. Coalesce or discard only explicitly cosmetic updates while
preserving all correctness events and exposing queue-pressure counters.

## Dependencies

- PERF-IPC-001 — DONE

## Definition of done

- A slow client cannot block the daemon, BitBake ingestion, or another client.
- Every client outbound queue has an explicit fixed bound.
- Progress, telemetry, and ordinary logs may coalesce under pressure.
- Errors, warnings, failures, cancellation, terminal outcomes, backend
  disconnects, required capability changes, and PTY control/output are never
  silently dropped.
- Queue depth, high-water mark, coalescing, resync, and disconnect pressure are
  observable.
- Deterministic tests prove critical ordering, bounded memory, slow-client
  isolation, and reconnect recovery.

## Verification

```bash
./scripts/verify-ipc-continuity.sh --backpressure
./scripts/verify-roadmap.sh
```

Measured incremental IPC, shared frame encoding, conservative snapshot sizing,
and bounded live replay are complete in v0.1.34.
