# Current Task

## Task

**ID:** METRICS-MODEL-001
**Title:** Audit all currently available telemetry
**Status:** IN_PROGRESS

## Objective

Identify every telemetry value Yoctui can currently obtain honestly and record
its authority, units, precision, sampling limits, host support, and unavailable
behavior before adding new metric visualizations.

## Dependencies

- `FOUNDATION-UI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- CPU usage/core count, memory, build filesystem, disk I/O, network I/O,
  daemon uptime/state, BitBake state, clients, sessions, and jobs are audited.
- Every supported metric records its authoritative source and exact units.
- Sampling interval, precision, reset/wrap behavior, and boundedness are
  documented where applicable.
- Unsupported or unavailable metrics have explicit host/runtime behavior.
- No renderer gains a fabricated or newly inferred value during this audit.

## Verification

```bash
cargo test -p yoctui-model telemetry_provenance
cargo test -p yoctui telemetry_provenance
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
