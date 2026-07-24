# Current task

## Active task

**ID:** RECIPES-META-001
**Title:** Add authoritative typed recipe metadata

## Objective

Extend the typed recipe metadata path from the live BitBake bridge through the
protocol/backend boundary into model state, preserving unavailable values
honestly and without parsing backend text in widgets.

## Required work

1. Inventory current recipe, source, task, build-history, dependency, Devtool,
   protocol, bridge, fake-backend, and live-harness representations.
2. Define backward-compatible typed recipe metadata for preferred/resolved
   version, provider layer, append count, workspace/Devtool status, build
   status, tasks, source paths, patches, package outputs, and history.
3. Populate fields from authoritative Tinfoil/BitBake data where available and
   use explicit unavailable/unknown values where the active backend cannot
   supply them.
4. Preserve compatibility with older bridge events through serde defaults and
   safe unknown capability handling.
5. Normalize wire data at the backend boundary and update model state only
   through typed reducer actions.
6. Add protocol round-trip, adapter conversion, reducer refresh, fake bridge,
   missing-data, and bridge query tests named `recipe_metadata`.
7. Add live-Yocto validation for the metadata actually claimed by the bridge;
   do not claim unavailable fields from mocked data.

## Definition of done

- Recipe metadata crosses every boundary as typed data.
- Missing fields remain visibly unknown/unavailable rather than fabricated.
- Refresh replaces authoritative metadata without leaving stale selections or
  detail records.
- Older/partial bridge payloads remain safe.
- Mocked coverage and required live metadata validation are distinct.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-protocol recipe_metadata
cargo test -p yoctui-bitbake recipe_metadata
cargo test -p yoctui-model recipe_metadata
python3 -m pytest bridge/tests -k recipe_metadata
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`RECIPES-UI-001 — Complete searchable Recipes Inspector`
