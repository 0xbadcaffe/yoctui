# Current Task

## Task

**ID:** METRICS-UI-002
**Title:** Add RAM gauge
**Status:** IN_PROGRESS

## Objective

Render authoritative host memory capacity as a responsive semantic gauge with
an honestly derived whole percentage and appropriately rounded used/total byte
values, without treating an invalid or unavailable sample as zero.

## Dependencies

- `METRICS-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Valid total/available memory renders an honestly derived whole used
  percentage with used and total byte values when width permits.
- Displayed byte precision does not exceed the precision of the sampled byte
  counters or imply a more exact percentage than the model derives.
- Wide and medium layouts use a semantic determinate gauge.
- Narrow layouts use a bounded horizontal or compact numeric fallback.
- Missing, zero-total, or inconsistent samples render unavailable rather than
  `0%` or synthetic capacity.
- High-contrast, no-color, and reduced-motion presentations retain meaningful
  text and visible state.

## Verification

```bash
cargo test -p yoctui-ui next_generation_ram_gauge
cargo test -p yoctui-model bounded_telemetry_history
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
