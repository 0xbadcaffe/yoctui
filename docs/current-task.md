# Current Task

## Task

**ID:** PERF-IPC-GATE-001
**Title:** Gate IPC continuity and critical-event ordering
**Status:** IN_PROGRESS

## Objective

Complete the default IPC continuity gate across deterministic event flood and
full-CPU saturation, including strict critical-event order, slow-client
isolation, and reconnect.

## Dependencies

- PERF-IPC-LATENCY-001 — DONE
- PERF-EVENT-FLOOD-001 — DONE

## Definition of done

- Full-affinity CPU saturation and a >=4,000 event/s production-path flood run
  through the default gate without client or backend disconnect.
- A healthy client retains strict protocol order and all warning, error,
  failure, cancellation, terminal, disconnect, and capability-critical
  evidence.
- A non-reading client cannot block daemon ingestion or the healthy client and
  is isolated through the bounded per-client policy.
- Queue pressure counters and bounds are validated from typed runtime state.
- Detach, reconnect, and authoritative snapshot recovery remain functional.
- The default gate runs without network access and does not depend on a real
  Poky workspace.

## Verification

```bash
./scripts/verify-ipc-continuity.sh
./scripts/verify-roadmap.sh
```

The saturation responsiveness gate is complete in v0.1.44.
