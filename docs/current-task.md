# Current Task

## Task

**ID:** METRICS-UI-005
**Title:** Add network sparklines
**Status:** IN_PROGRESS

## Objective

Render optional authoritative reset-aware network receive and transmit rates
with bounded recent history as semantic sparklines, without making network
telemetry a requirement for core functionality or displaying unavailable
values as zero.

## Dependencies

- `METRICS-UI-004` — DONE
- `METRICS-MODEL-002` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Current RX and TX rates render separately with byte-per-second units.
- Each sparkline consumes only its bounded typed history and scales to the
  retained maximum without mutating or reparsing sampler data.
- First/reset/interface-change/overflow/unavailable observations do not render
  as a spike or synthetic zero.
- Hosts without a supported active interface keep network telemetry optional
  and explicitly unavailable or omitted according to layout space.
- Narrow rendering keeps meaningful text without panic or overlap.
- High-contrast, no-color, and reduced-motion presentations retain meaningful
  text and visible state.

## Verification

```bash
cargo test -p yoctui-ui next_generation_network_io
cargo test -p yoctui telemetry_sampling
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
