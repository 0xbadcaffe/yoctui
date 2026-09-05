# Current Task

## Task

**ID:** PERF-SCHED-001
**Title:** Evaluate safe unprivileged scheduling recommendations
**Status:** IN_PROGRESS

## Objective

Measure whether unprivileged nice levels or systemd user-service CPUWeight
materially improve Yoctui responsiveness under CPU saturation. Publish only
optional, measured guidance; require neither privilege nor real-time policy.

## Dependencies

- PERF-BASELINE-001 — DONE
- PERF-SATURATION-HARNESS-001 — DONE

## Definition of done

- Baseline and adjusted nice behavior are measured with the deterministic full-
  affinity CPU fixture using a documented, repeatable method.
- User-service CPUWeight applicability and limitations are documented from the
  installed system interfaces without requiring a user service manager.
- No root, real-time policy, or mandatory priority change is introduced.
- Any recommendation is optional, safe, and tied to recorded evidence.
- Yoctui remains correct and responsive at its normal inherited priority.

## Verification

```bash
./scripts/verify-performance.sh --scheduling
./scripts/verify-roadmap.sh
```

Tokio runtime and blocking-pool scheduling, stable thread bounds, and reactor
progress under full-affinity CPU saturation are complete in v0.1.37.
