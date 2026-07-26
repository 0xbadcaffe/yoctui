# Current task

## Active task

**ID:** PKG-MODEL-001
**Title:** Add typed package data state

## Objective

Define pure bounded package inventory/detail state, exact identities,
deterministic normalization and dependency navigation, and reducer lifecycle
without parsing `oe-pkgdata-util` output.

## Required work

1. Inventory existing recipe/package fields, image state, selection/search
   conventions, dependency graph helpers, backend events, and effects before
   adding overlapping behavior.
2. Define an exact runtime-package identity and typed summary/detail records.
   Represent recipe ownership, provider path, files, runtime dependencies,
   reverse dependencies, installed size, license, and image membership as
   explicit available or unavailable fields rather than guesses.
3. Normalize package inventories deterministically by exact identity, reject
   invalid records, retain a deterministic duplicate, and enforce hard bounds
   on packages plus all nested detail collections.
4. Represent not-loaded, loading, available-empty, available, partial, and
   failed inventory/detail states with exact request correlation.
5. Preserve selection by package identity across refresh, clamp safely when a
   package disappears, and keep detail responses cached/correlated so stale
   responses cannot replace current state.
6. Add pure case-insensitive search across package, recipe, and available
   ownership fields plus deterministic runtime and reverse-dependency
   navigation that resolves only identities present in the current inventory.
7. Add reducer actions/effects for inventory request/success/partial/failure,
   selection/search, detail request/success/partial/failure, refresh, and
   dependency navigation. Backend/UI code must not mutate model state.
8. Add typed backend-event mapping hooks and model/app tests named
   `pkgdata_model` for validation, duplicates, bounds, every explicit state,
   stable selection, stale correlation, unavailable fields, search, dependency
   navigation, and event/effect mapping.
9. Update `docs/architecture.md` for package state ownership and the future
   `oe-pkgdata-util` adapter boundary.

## Definition of done

- Pure model state owns exact bounded package inventory/detail data and
  lifecycle.
- Unavailable, empty, partial, failed, and stale outcomes remain distinct.
- Reducers consume typed data only and emit typed effects.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model pkgdata_model
cargo test -p yoctui-app pkgdata_model
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`PKG-ADAPTER-001 — Acquire authoritative package data`
