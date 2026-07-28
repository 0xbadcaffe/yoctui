# Current task

## Active task

**ID:** IMAGES-MODEL-001
**Title:** Add typed image artifact state

## Objective

Add pure, bounded domain state for authoritative deployed image artifacts
without filesystem access, raw-output parsing, or changes to the existing
image picker and confirmed build behavior.

## Required work

1. Inspect existing image picker/build state, typed package/signature state
   patterns, app event normalization, and tests before adding overlapping
   behavior.
2. Define exact artifact identity using machine, image target, and absolute
   deployed path. Reject relative or identity-mismatched records.
3. Represent artifact kind, byte size, modification timestamp, deploy path,
   checksum records, manifests, licenses, SPDX/SBOM outputs, and Wic-related
   files as typed available/unavailable data; do not infer meaning in widgets.
4. Bound record counts, associated-file counts, text metadata, and
   normalization reports. Sort and deduplicate deterministically.
5. Add explicit not-loaded, loading, available-empty, available, partial, and
   failed inventory states with request generation correlation.
6. Add identity-stable selection and case-insensitive search across exact typed
   fields. Preserve a selected identity across refresh when it still exists.
7. Add typed reducer actions/effects and app backend-event mapping for request,
   success, partial, failure, search, and selection. No reducer may read the
   filesystem.
8. Preserve existing `ImagePicker`, `BeginCurrentImageBuild`, and confirmed
   build behavior unchanged.
9. Add focused model and app tests named `image_artifact_model` for normal,
   empty, partial, failed, stale, bounded, and invalid-identity paths.
10. Update `docs/architecture.md` for the new typed ownership boundary, then
    update registry/status and hand off to `IMAGES-ADAPTER-001`.

## Definition of done

- The model owns bounded, deterministic, correlated artifact state and exact
  selection/search behavior.
- App mapping crosses only typed artifact events and effects.
- Existing image selection and build tests remain green.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model image_artifact_model
cargo test -p yoctui-app image_artifact_model
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
