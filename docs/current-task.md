# Current Task

## Task

**ID:** PERF-INPUT-LATENCY-001
**Title:** Measure saturated UI input latency
**Status:** IN_PROGRESS

## Objective

Measure keyboard input to reducer action, keyboard input to visible frame, and
mouse input to visible selection under deterministic full-CPU saturation.

## Dependencies

- PERF-SATURATION-HARNESS-001 — DONE
- PERF-RENDER-001 — DONE

## Definition of done

- At least 100 post-warmup observations exist for keyboard-to-model,
  keyboard-to-visible-frame, and mouse-to-visible-selection latency.
- Measurements use monotonic timestamps while every affinity CPU is runnable.
- Each path reports p50/p95 and meets the documented p95 <=100 ms target.
- Keyboard and mouse handling remain independent of animation/render ticks.
- Evidence identifies the exact revision, host, terminal, load, and method.

## Verification

```bash
./scripts/verify-saturation-responsiveness.sh --input-latency
./scripts/verify-roadmap.sh
```

BitBake coexistence guidance is complete in v0.1.40.
