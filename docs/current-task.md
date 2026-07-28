# Current task

## Active task

**ID:** IMAGES-001
**Title:** Complete Images artifact workspace

## Objective

Turn the existing image-recipe selection/build screen into an authoritative
artifact workspace for deployed images and their associated metadata.

## Required work

1. Inspect the existing Images model, UI, build actions, deploy-directory
   discovery, backend metadata, tests, and documentation before changing code.
2. Reconcile this broad parent task into atomic child tasks if it cannot be
   completed as one coherent verified commit.
3. Preserve existing image recipe selection and confirmed build behavior.
4. Acquire deployed image artifacts from the authoritative build deploy
   directory with exact machine/image identities, bounded filesystem access,
   and explicit loading, empty, partial, failed, and unavailable states.
5. Represent artifact file sizes and timestamps plus manifests, licenses,
   checksums, SPDX/SBOM outputs, Wic images, and deploy locations as typed data.
6. Add identity-stable artifact selection, search/filtering, responsive
   Workspace/Inspector rendering, contextual open actions, footer hints, and
   honest disabled explanations.
7. Keep scanning and expensive metadata work outside widgets and reducers.
8. Add model, app, adapter/CLI, and Ratatui TestBackend tests as applicable.
9. Update `docs/ui-spec.md` with every intentional UI behavior and
   `docs/architecture.md` with component-boundary changes.

## Definition of done

- Image recipe builds and deployed artifact inspection coexist in one
  responsive typed workspace.
- Artifact identities and metadata are authoritative, bounded, and explicit
  about unavailable or partial results.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model images_workspace
cargo test -p yoctui-ui images_workspace
cargo test -p yoctui-app image_action
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
