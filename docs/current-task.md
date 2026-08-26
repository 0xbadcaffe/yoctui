# Current Task

## Task

**ID:** UX-TELEMETRY-001
**Title:** Polish telemetry gauges meters and zoomed charts
**Status:** NOT_STARTED

## Objective

Use existing typed histories for compact semantic meters and expanded charts
with exact units, honest missing samples, responsive collapse, and safe zoom.

## Dependencies

- `UX-WIDGET-PRIMITIVES-001` — DONE

## Relevant files

- typed host telemetry and bounded histories
- telemetry renderers and semantic graph roles
- pane subfocus/zoom presentation
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Existing typed histories drive compact sparklines and expanded charts.
- Every current value retains exact units and missing samples remain gaps.
- CPU, RAM, filesystem, I/O, and network roles remain semantically distinct.
- Zoom is safe and responsive collapse preserves textual values.
- Empty, partial, unavailable, large, and Unicode inputs never panic.

## Verification

```bash
cargo test -p yoctui-model ux_telemetry
cargo test -p yoctui-ui ux_telemetry
./scripts/test-next-generation-ui-performance.sh
./scripts/verify-roadmap.sh
```
