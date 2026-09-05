# Current Task

## Task

**ID:** PERF-LOG-001
**Title:** Optimize bounded log ingestion and rendering
**Status:** IN_PROGRESS

## Objective

Keep high-rate log ingestion bounded and responsive without cloning,
normalizing, filtering, or redrawing the full retained history per line or per
frame. Preserve warnings, errors, failures, and exact correlation.

## Dependencies

- PERF-EVENT-FLOOD-001 — DONE
- PERF-RENDER-001 — DONE

## Definition of done

- Ordinary high-rate logs are ingested in bounded batches and can be coalesced
  without delaying critical warnings/errors/failures.
- Retained log entry/byte bounds hold under sustained floods.
- Rendering does not clone or lowercase the full history every frame.
- Search/filter work is incremental or cached and invalidated only by relevant
  log/query changes.
- A burst of ordinary lines requests bounded render work rather than one frame
  per line.
- Focused tests and `verify-performance.sh --logs` cover retention, critical
  priority, batching, cached filtering, and render coalescing.

## Verification

```bash
./scripts/verify-performance.sh --logs
./scripts/verify-roadmap.sh
```

Demand-aware bounded telemetry is complete in v0.1.31.
