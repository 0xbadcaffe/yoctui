# Current Task

## Task

**ID:** PERF-RENDER-001
**Title:** Make rendering dirty and event driven
**Status:** IN_PROGRESS

## Objective

Complete the measured rendering optimization by giving every meaningful model
mutation an explicit, coalesced invalidation path and rendering only when the
visible frame can change. Record render attempts, rendered frames, and skipped
identical requests without coupling keyboard processing to render cadence.

## Dependencies

- PERF-WAKEUPS-001 — DONE
- PERF-EVENTLOOP-001 — DONE

## Definition of done

- Input, meaningful daemon/backend/local state changes, resize, and due visible
  timers request a frame.
- Multiple requests before a frame are coalesced.
- Unchanged state and hidden timers do not render identical frames.
- Elapsed-time refresh is at most 1 Hz when reduced motion removes animation.
- Active normal-build rendering remains at most 10 frames/s; PTY publication
  retains its separate 30 frames/s contract.
- Focused deterministic tests expose render request/render/skip rates and prove
  input-to-frame remains independent of periodic ticks.
- `verify-performance.sh --render` rejects unconditional or duplicate redraws.

## Verification

```bash
./scripts/verify-performance.sh --render
./scripts/verify-roadmap.sh
```

The listener and idle-loop gate are complete in v0.1.28. This task builds on
the initial redraw latch by making its invalidation and measurements explicit.
