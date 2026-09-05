# Current Task

## Task

**ID:** PERF-ANIM-001
**Title:** Bound animation work and cadence
**Status:** IN_PROGRESS

## Objective

Make indeterminate activity animation visible-only, low-frequency, and
independent of unrelated application state. Freeze it in reduced-motion mode
and ensure hidden or terminal activity cannot request frames.

## Dependencies

- PERF-RENDER-001 — DONE

## Definition of done

- Animation cadence is explicitly bounded between 4 and 10 Hz.
- Only a screen/dialog containing visible indeterminate activity advances the
  animation frame.
- Hidden, determinate, and terminal work does not schedule animation frames.
- Reduced-motion mode freezes the animation frame while preserving lifecycle
  text and elapsed-time updates at no more than 1 Hz.
- Animation state changes do not force unrelated full-app work.
- Focused model/UI/runtime tests and `verify-performance.sh --animations`
  reject cadence, visibility, terminal-state, and reduced-motion regressions.

## Verification

```bash
./scripts/verify-performance.sh --animations
./scripts/verify-roadmap.sh
```

Dirty rendering and its exact counters are complete in v0.1.29.
