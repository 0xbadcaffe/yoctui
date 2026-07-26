# Current task

## Active task

**ID:** PKG-UI-001
**Title:** Integrate the package data workspace

## Objective

Add a responsive Packages Navigator workspace that acquires typed package data
in the background and supports search, stable selection, detail inspection,
dependency navigation, refresh, and authoritative provider/recipe actions.

## Required work

1. Inspect the existing Packages Navigator placeholder, responsive workspace
   patterns, footer hints, background signature coordination, package model,
   and adapter before adding overlapping behavior.
2. Expand `docs/ui-spec.md` before implementing the exact Packages layouts,
   focus behavior, explicit state rendering, shortcuts, and disabled reasons.
3. Route the existing Packages Navigator entry to a typed package workspace.
   Entering or refreshing starts a correlated background inventory request
   without blocking terminal drawing or input.
4. Render search and identity-stable package selection with package name,
   recipe, version, size, license, and explicit unavailable values. Never infer
   data from raw adapter output.
5. Load the selected package detail lazily and render files, runtime
   dependencies, reverse dependencies, and image membership with distinct
   not-loaded, loading, available-empty, available, partial, and failed states.
6. Provide typed forward/reverse dependency navigation and return navigation,
   preserving selection by exact package identity. Missing inventory identities
   remain disabled with an explanation.
7. Provide typed refresh plus provider/recipe navigation only when an
   authoritative path or recipe identity is available. Do not fabricate a
   provider path.
8. Keep package acquisition in a cancellable CLI-owned Tokio task. Convert
   responses/errors to correlated typed backend events/actions; leaving the
   workspace must not corrupt or misapply a pending result.
9. Show package-specific footer hints and contextual disabled reasons. Search
   editing and every dialog or overlay must trap focus consistently.
10. Render wide, medium, narrow, too-small, light, dark, and no-color modes
    without panic. Partial/failure/empty/loading/unavailable states must remain
    semantically visible without color.
11. Add model reducer, app input/effect mapping, CLI background integration, and
    Ratatui `TestBackend` coverage named `pkgdata_workspace`.
12. Update `docs/architecture.md` for CLI coordination and
    `docs/implementation-status.md` plus `docs/task-registry.toml` after all
    focused and baseline checks pass.

## Definition of done

- Packages is a usable responsive workspace backed only by typed package data.
- Inventory/detail acquisition remains asynchronous, cancellable, correlated,
  and honest about unavailable or missing generated pkgdata.
- Search, selection, refresh, details, dependency navigation, and contextual
  actions are reducer-owned and tested at every supported breakpoint.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model pkgdata_workspace
cargo test -p yoctui-app pkgdata_workspace
cargo test -p yoctui-ui pkgdata_workspace
cargo test -p yoctui -- pkgdata_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
