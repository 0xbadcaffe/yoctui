# Current Task

## Task

**ID:** RAW-FAVORITE-PERSIST-001
**Title:** Persist Raw favorites atomically
**Status:** IN_PROGRESS

## Objective

Load and save bounded versioned Raw favorites through the existing user-local
atomic session-state boundary without granting persisted execution authority.

## Dependencies

- `RAW-FAVORITE-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The versioned session schema stores Raw favorites only in user-local
  `session.toml`, using the existing private atomic replacement path.
- Loading validates schema, identities, names, typed defaults, argv, ordering,
  record count, and aggregate bytes before replacing model state.
- Malformed, duplicate, unknown-version, or oversized favorite data fails
  closed without partially installing records or disturbing unrelated session
  preferences.
- Removed or changed catalog templates load as explicit stale favorites rather
  than being discarded or upgraded.
- Persistence never includes process/session/job identity, output, executable
  or build authority, capability generation/state, preview/request identity,
  or transient form state.
- CLI tests cover round trip, atomic replacement, legacy/default loading,
  malformed and oversized rejection, stale retention, permissions, and
  preservation of unrelated session fields.

## Verification

```bash
cargo test -p yoctui -- raw_favorite_persistence
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
