# Current Task

## Task

**ID:** PERF-RESPONSIVENESS-GATE-001
**Title:** Gate interactive responsiveness under saturation
**Status:** IN_PROGRESS

## Objective

Implement the deterministic full-CPU saturation gate for continued keyboard,
mouse, rendering, daemon/backend connection, cancellation, detach, and
reconnect responsiveness.

## Dependencies

- PERF-INPUT-LATENCY-001 — DONE
- PERF-BITBAKE-CONN-001 — DONE

## Definition of done

- Every CPU in the caller affinity remains runnable under the deterministic
  load fixture; no core is deliberately reserved.
- Real PTY keyboard and mouse actions remain visible with p95 <=100 ms and
  rendering continues.
- The daemon/client and production-path fixture backend remain connected across
  the saturated observation window.
- Cancellation is acknowledged and reaches a terminal result without being
  starved by cosmetic work.
- Detach and fresh reconnect both succeed under load without losing critical
  ordering.
- The offline gate retains exact host, binary/source, load, raw latency, render,
  connection, cancellation, detach, and reconnect evidence.

## Verification

```bash
./scripts/verify-saturation-responsiveness.sh
./scripts/verify-roadmap.sh
```

The one-percent steady-state CPU gate is complete in v0.1.43.
