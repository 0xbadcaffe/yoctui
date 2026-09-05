# Current Task

## Task

**ID:** PERF-EVENTLOOP-001
**Title:** Eliminate idle busy loops
**Status:** IN_PROGRESS

## Objective

Use the captured baseline, profiles, and wakeup audit to make every long-lived
Yoctui loop block efficiently when idle. Remove unconditional short polling,
spinning receives, and unchanged periodic work without changing event ordering,
input correctness, cancellation, PTY ownership, or daemon/client liveness.

## Dependencies

- PERF-FLAMEGRAPH-001 — DONE
- PERF-WAKEUPS-001 — DONE

## Definition of done

- The daemon listener and supervisor loop blocks when there are no clients,
  jobs, PTYs, pending operations, or due telemetry samples.
- Client input/IPC waiting does not spin and does not force redraw by itself.
- Every long-lived `try_recv`, poll, reconnect, PTY, log/job/status, and timer
  loop has an explicit blocking or bounded-notification contract.
- No zero-duration sleep, repeated unchanged metadata/capability/filesystem
  probing, or idle redraw remains.
- Focused tests measure bounded idle wakeups and prove prompt notification for
  daemon commands, backend events, input, PTY output, and shutdown.
- The performance verifier rejects regressions to the audited busy-loop forms.

## Verification

```bash
./scripts/verify-performance.sh --event-loops
./scripts/verify-roadmap.sh
```

Baseline/profile/wakeup evidence and both deterministic fixtures are complete.
This is the first runtime optimization task.
