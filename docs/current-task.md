# Current Task

## Task

**ID:** PERF-EVENT-FLOOD-001
**Title:** Create a BitBake-like event flood harness
**Status:** IN_PROGRESS

## Objective

Create a deterministic BitBake-like event producer that drives the production
bridge, daemon reducer/journal, IPC, and client paths above expected build
rates. It must mix task lifecycle/progress, ordinary logs, warnings, errors,
failure, cancellation, backend disconnect, and terminal outcomes while making
rate, ordering, retention, memory, and connection evidence measurable.

## Dependencies

- PERF-SPEC-001 — DONE

## Definition of done

- The fixture emits realistic queued, started, progress, log, warning, error,
  completed, failed/cancelled, and backend lifecycle events with stable task IDs.
- Configurable rates include at least the contractual 2,000 events/s and exceed
  expected production rates without network or a Yocto checkout.
- Critical sequence sent/received/retained evidence proves that ordinary traffic
  cannot invent loss of terminal, failure, cancellation, or disconnect events.
- Runtime duration, actual event rates, queue/retention bounds, memory, client
  continuity, and terminal completion are machine-readable.
- Tests cover event mix, deterministic ordering, invalid configuration, bounded
  exit, and the currently failing pre-backpressure behavior honestly.

## Verification

```bash
./scripts/verify-ipc-continuity.sh --event-flood
./scripts/verify-roadmap.sh
```

The pre-optimization evidence and deterministic CPU saturation fixture are
complete. No runtime optimization has landed.
