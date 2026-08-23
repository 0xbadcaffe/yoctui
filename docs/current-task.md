# Current Task

## Task

**ID:** RAW-FAVORITE-MODEL-001
**Title:** Define persistent Raw favorite model
**Status:** IN_PROGRESS

## Objective

Define a bounded, versioned Raw favorite model that stores reusable command
configuration intent while keeping execution and capability authority live and
reviewed.

## Dependencies

- `RAW-MODEL-001` — DONE
- `RAW-PARAM-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- A versioned favorite record retains stable command identity, a bounded
  user-visible name, validated typed parameter defaults, validated additional
  arguments, and explicit ordering.
- Favorite records never retain PID, process/session/job identity, output,
  executable/build authority, capability generation, preview digest, secret,
  or transient form/execution state.
- Add, update, remove, rename, and reorder operations are deterministic,
  identity-safe, count/byte bounded, and reject malformed or unknown-version
  records.
- Projection against the current catalog preserves removed or changed commands
  as explicit stale favorites and reports current five-state compatibility
  without granting execution authority.
- Reopening a valid favorite creates fresh form defaults only; execution still
  requires current capability validation, exact preview, confirmation, and a
  new request identity.
- Model tests cover validation, bounds, duplicate identity, editing, ordering,
  stale catalog/template projection, and fresh-form reconstruction.

## Verification

```bash
cargo test -p yoctui-model raw_favorite
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
