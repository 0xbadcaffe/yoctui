# Current Task

## Task

**ID:** METRICS-UI-001
**Title:** Add CPU gauge
**Status:** IN_PROGRESS

## Objective

Render the authoritative host CPU utilization and logical core count as a
compact semantic gauge that degrades to a simpler horizontal presentation on
narrow terminals and never represents unavailable data as zero.

## Dependencies

- `METRICS-MODEL-002` — DONE
- `FOUNDATION-UI-003` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Current host CPU utilization is always accompanied by a numeric percentage.
- Logical core count renders only when authoritative.
- Wide and medium layouts use the semantic compact gauge treatment.
- Narrow layouts use a bounded horizontal or compact fallback without panic.
- Missing utilization renders unavailable rather than `0%`.
- High-contrast, no-color, and reduced-motion presentations retain meaningful
  text and visible state.

## Verification

```bash
cargo test -p yoctui-ui next_generation_cpu_gauge
cargo test -p yoctui-model telemetry_provenance
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
