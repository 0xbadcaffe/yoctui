# Current Task

## Task

**ID:** METRICS-MODEL-002
**Title:** Add bounded telemetry history model
**Status:** IN_PROGRESS

## Objective

Add one bounded model for recent telemetry samples, extending the existing
CPU/RAM retention to supported disk-I/O and network rates without storing
invalid, reset, or unavailable observations.

## Dependencies

- `METRICS-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- CPU, memory, supported disk-I/O, and supported network samples share an
  explicit fixed capacity.
- Rate histories accept only values derived from two valid monotonic counters
  and a nonzero measured interval.
- Counter reset, interface disappearance, overflow, and unavailable samples do
  not append spikes or synthetic zeroes.
- Every history remains bounded under prolonged sampling.
- Existing CPU/RAM history behavior and typed reducer ownership are preserved.

## Verification

```bash
cargo test -p yoctui-model bounded_telemetry_history
cargo test -p yoctui telemetry_sampling
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
