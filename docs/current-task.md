# Current Task

## Task

**ID:** PERF-TELEMETRY-001
**Title:** Optimize telemetry collection
**Status:** IN_PROGRESS

## Objective

Reduce host and daemon telemetry work to a low, demand-aware cadence without
per-sample process creation. Reuse kernel sources where practical and retain
only bounded histories.

## Dependencies

- PERF-EVENTLOOP-001 — DONE
- PERF-RENDER-001 — DONE

## Definition of done

- CPU, memory, disk, network, filesystem, and daemon-health collection cadences
  are explicit, low-frequency, and measured.
- No telemetry sample spawns an external process.
- Reusable procfs/sysfs handles or cached static metadata avoid repeated setup
  where measurement shows value.
- Invisible telemetry is paused or reduced when no attached client needs it.
- Histories remain bounded and unchanged samples do not request redundant
  client frames or daemon publications.
- Focused tests and `verify-performance.sh --telemetry` reject sampling-rate,
  process-spawn, visibility, and retention regressions.

## Verification

```bash
./scripts/verify-performance.sh --telemetry
./scripts/verify-roadmap.sh
```

Visible-only 5 Hz animation and separate 1 Hz elapsed refresh are complete in
v0.1.30.
