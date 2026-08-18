# Current Task

## Task

**ID:** COMPAT-CAP-MODEL-001
**Title:** Create centralized typed capability model
**Status:** IN_PROGRESS

## Objective

Create the pure, bounded, behavior-oriented capability domain model that will
become the single availability source for catalog evaluation, daemon state,
protocol snapshots, actions, and rendering.

## Dependencies

- `COMPAT-ENV-ID-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Stable typed capability IDs represent behavior, not release versions.
- States are Available, AvailableWithLimitations, Unavailable, Unknown, and
  Unsupported with bounded reason codes/text and typed evidence.
- A normalized snapshot is tied to an exact environment identity and monotonic
  generation, rejects duplicates/invalid evidence, and supports lookup.
- The required initial capability inventory is centralized outside renderers.
- Tests cover all states, reason/evidence bounds, duplicates, ordering,
  environment association, and unknown-safe action decisions.

## Verification

```bash
cargo test -p yoctui-model compatibility::capability
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
