# Current task

## Active task

**ID:** IMAGES-UI-001
**Title:** Integrate the responsive Images artifact workspace

## Objective

Combine the existing image recipe picker/build flow with asynchronous deployed
artifact browsing and inspection in the persistent responsive workbench.

## Required work

1. Inspect the existing Images rendering, persistent-shell responsive helpers,
   image picker/build routing, package/signature background coordinators, model,
   adapter, and tests before adding overlapping behavior.
2. Expand `docs/ui-spec.md` first with exact wide/medium/narrow layouts,
   focus/selection behavior, explicit lifecycle states, shortcuts, Inspector
   sections, contextual actions, and disabled explanations.
3. Keep image recipe discovery and the `i` picker. In Images, `b` must build
   the selected artifact's exact image target when one is selected, otherwise
   preserve the existing current-image confirmation behavior.
4. Entering Images starts one correlated artifact scan only from not-loaded
   state. `R` refreshes, `c` cancels, and leaving the workspace never applies a
   stale response.
5. Keep the adapter in one cancellable CLI-owned Tokio task. Derive its
   configured path only from typed `DEPLOY_DIR_IMAGE`; missing/invalid values
   produce an explicit failed/unavailable state without blocking input.
6. Add app-owned input mapping for artifact selection, search editing, refresh,
   cancellation, exact image-target build selection, and authoritative open
   actions. Dialogs continue to trap focus.
7. Render recipe targets and deployed artifacts together without conflating
   them. Show exact machine, kind, file name/path, size, timestamp, deploy
   directory, checksums, manifests, licenses, SPDX/SBOM, Wic files, and all
   limitations using only typed state.
8. Render distinct not-loaded, loading, available-empty, available, partial,
   failed, unavailable-field, and no-search-match states. No-color mode must
   preserve semantics with text/attributes.
9. Use the persistent shell: wide shows artifact list plus Inspector, medium
   uses the existing Inspector overlay, narrow uses the visible pane switcher,
   and too-small terminals use the shared safe message.
10. Add contextual open-in-editor actions only for exact selected deployed
    paths and associated paths. Missing selections/fields show stable disabled
    reasons; never fabricate or parse paths in UI/app code.
11. Add reducer, input, CLI integration, and Ratatui `TestBackend` tests named
    `images_workspace`, covering background success/partial/failure/cancel,
    build preservation, search/selection, open actions, all responsive modes,
    light/dark/no-color, and narrow lifecycle states.
12. Update `docs/architecture.md` for CLI coordination, then update
    registry/status and hand off to the `IMAGES-001` parent verification.

## Definition of done

- Existing image picker and confirmed builds coexist with a usable typed
  artifact workspace.
- Scanning remains asynchronous, cancellable, correlated, and derived only
  from authoritative workspace variables.
- Responsive state, selection, inspection, search, build, and open actions are
  tested without raw parsing in widgets.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model images_workspace
cargo test -p yoctui-app images_workspace
cargo test -p yoctui-ui images_workspace
cargo test -p yoctui -- images_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
