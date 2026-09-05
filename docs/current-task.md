# Current Task

## Task

**ID:** PERF-MEMORY-GATE-001
**Title:** Gate bounded memory under sustained event load
**Status:** IN_PROGRESS

## Objective

Prove that sustained high-rate logs, task updates, IPC fanout, telemetry, and
PTY traffic remain bounded without monotonic process-memory or thread growth.

## Dependencies

- PERF-LOG-001 — DONE
- PERF-TASKS-001 — DONE
- PERF-IPC-BACKPRESSURE-001 — DONE

## Definition of done

- A deterministic sustained high-rate workload exercises logs, tasks, daemon
  journal/client queues, telemetry histories, and PTY retention.
- All model and transport collections remain at their documented hard bounds.
- Daemon and attached-client RSS do not grow by more than 32 MiB over the
  bounded gate, and retained endurance evidence has a final-window slope no
  greater than 64 KiB/min.
- Thread counts remain stable and no correctness-critical terminal record is
  lost under retention pressure.
- The default verifier runs without network access and distinguishes the
  deterministic gate from retained long-duration evidence.

## Verification

```bash
./scripts/verify-bounded-memory.sh
./scripts/verify-roadmap.sh
```

The IPC continuity gate is complete in v0.1.45.
