# Current Task

## Task

**ID:** METRICS-UI-003
**Title:** Add disk and build-filesystem gauge
**Status:** IN_PROGRESS

## Objective

Render authoritative capacity for the filesystem backing the configured build
directory as a responsive semantic gauge with honest used/free values and
clear build-filesystem context.

## Dependencies

- `METRICS-UI-002` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Valid total/available build-filesystem capacity renders an honestly derived
  whole used percentage and free/total or used/total values when width permits.
- The configured build directory or an unambiguous build-filesystem label
  identifies the exact sampled context without inventing a device name.
- Wide and medium layouts use a semantic determinate gauge; narrow layouts use
  a bounded horizontal or compact numeric fallback.
- Missing build directory, missing sample, zero total, or inconsistent capacity
  renders unavailable rather than `0%` or synthetic capacity.
- High-contrast, no-color, and reduced-motion presentations retain meaningful
  text and visible state.

## Verification

```bash
cargo test -p yoctui-ui next_generation_disk_gauge
cargo test -p yoctui telemetry_sampling
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
