# Current task

## Active task

**ID:** CONFIG-UI-001
**Title:** Complete searchable Configuration Inspector

## Objective

Turn the existing effective-variable table into a responsive, searchable
Configuration workspace with lazy authoritative detail and explicit partial
states.

## Required work

1. Inventory current variable selection/search, global and recipe-scoped
   identity, detail storage, bridge effects, CLI execution, rendering, and
   tests without duplicating existing behavior.
2. Make `Enter` lazily request detail for the selected variable using a stable
   `VariableIdentity`; correlate completion by that identity so stale or
   recipe-scoped responses cannot replace the selected global detail.
3. Model loading and error states separately from unavailable, available-empty,
   and populated detail.
4. Render effective and unexpanded values, scope, provenance, defining
   file/line operations, overrides, and append/prepend/remove history without
   parsing raw backend data in widgets.
5. Preserve bounded selection through search/filter changes and render safely
   in wide, medium, narrow, empty, partial, and failed states.
6. Add reducer, app/input, CLI integration, and Ratatui TestBackend tests named
   `config_workspace`.
7. Update `docs/ui-spec.md` for the intentional interaction and responsive
   behavior; update architecture only if component boundaries change.

## Definition of done

- Search and selection remain identity-stable and bounded.
- `Enter` requests typed lazy detail and stale responses remain isolated.
- Every required detail and partial/failure state renders honestly.
- Responsive Configuration layouts never panic or fabricate metadata.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model config_workspace
cargo test -p yoctui-app config_workspace
cargo test -p yoctui-ui config_workspace
cargo test -p yoctui -- config_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-ACTIONS-001 — Add typed configuration copy, source, and compare actions`
