# Current Task

## Task

**ID:** PERF-BITBAKE-CONN-001
**Title:** Harden BitBake connectivity under CPU saturation
**Status:** IN_PROGRESS

## Objective

Audit BitBake socket deadlines, heartbeat/liveness assumptions, blocking
operations, retry timing, and read/write starvation. Keep the backend alive
under severe scheduler delay without hiding a real disconnect.

## Dependencies

- PERF-IPC-001 — DONE
- PERF-EVENTLOOP-001 — DONE

## Definition of done

- Liveness and timeout decisions use monotonic time.
- Scheduler delay or one ordinary timeout cannot be interpreted as backend
  death.
- Read/write waits and post-terminal cleanup are explicitly bounded without
  starving native events or cancellation.
- Heartbeat and reconnect policy tolerate sustained full-CPU contention while
  still detecting a real EOF/disconnect.
- Deterministic full-CPU tests prove backend continuity, real disconnect
  reporting, and cancellation acknowledgement.

## Verification

```bash
./scripts/verify-saturation-responsiveness.sh --bitbake-connection
./scripts/verify-roadmap.sh
```

Bounded priority ingress, slow-client isolation, pressure counters, and strict
flood/reconnect verification are complete in v0.1.35.
