# Current Task

## Task

**ID:** PERF-IPC-001
**Title:** Audit and optimize daemon/client IPC
**Status:** IN_PROGRESS

## Objective

Measure message and byte rates, snapshot and incremental-event sizes,
serialization cost, and fanout behavior. Remove redundant snapshots or
serialization while retaining ordered incremental correctness events.

## Dependencies

- PERF-BASELINE-001 — DONE
- PERF-EVENT-FLOOD-001 — DONE

## Definition of done

- Reproducible artifacts record messages/s, bytes/s, snapshot size,
  incremental sizes, and serialization CPU/cost.
- Normal state changes use incremental events rather than redundant snapshots.
- Identical per-client payload serialization is shared where practical.
- Ordering and resume/gap semantics remain exact.
- Focused tests and `verify-performance.sh --ipc` validate the audit and
  implemented optimizations offline.

## Verification

```bash
./scripts/verify-performance.sh --ipc
./scripts/verify-roadmap.sh
```

Bounded batched task updates and cached task-table ordering are complete in
v0.1.33.
