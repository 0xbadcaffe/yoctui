# Current Task

## Task

**ID:** PERF-FLAMEGRAPH-001
**Title:** Capture CPU flamegraphs for all baseline workloads
**Status:** IN_PROGRESS

## Objective

Profile the exact pre-optimization release runtime for idle daemon, idle
client, active real build, log-heavy, task-event-heavy, PTY idle, and PTY active
workloads. Retain concise symbolized artifacts that identify actionable hot
paths before changing event loops, rendering, telemetry, IPC, or reducers.

## Dependencies

- PERF-BASELINE-001 — DONE

## Definition of done

- Every required workload has a named, reproducible sampling command.
- Profiles use production paths and preserve fixture versus real-Poky identity.
- Reports identify sample count, resolved-frame quality, dominant stacks, exact
  binary/revision, and artifact hashes.
- Raw `perf.data` remains reproducible and untracked; concise reports and
  flamegraphs follow `artifacts/performance/profiles/` policy.
- Findings name measured hot paths without treating hypotheses as fixes.

## Verification

```bash
./scripts/verify-performance.sh --profiles
./scripts/verify-roadmap.sh
```

The pre-optimization baseline is complete. No runtime optimization has landed.
