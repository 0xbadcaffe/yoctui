# Current task

## Active task

**ID:** IMAGES-ADAPTER-001
**Title:** Acquire authoritative deployed image artifacts

## Objective

Add a deterministic, bounded adapter that resolves configured Yocto deploy
locations and returns only typed image artifact inventory data.

## Required work

1. Inspect existing workspace variable acquisition, filesystem adapters,
   cancellation patterns, image naming conventions, and tests before adding
   overlapping behavior.
2. Add authoritative `DEPLOY_DIR_IMAGE` acquisition to live Tinfoil and
   environment workspace snapshots without locally evaluating BitBake syntax.
3. Require an absolute configured deploy directory and exact request machine.
   Canonicalize and constrain every traversed path; reject symlink escape,
   non-directory, relative, missing, and machine-mismatch inputs explicitly.
4. Scan deterministically without a shell and with strict entry, metadata-byte,
   recursion-depth, and elapsed-time bounds. Do not block a Ratatui widget or
   reducer.
5. Classify image/rootfs, kernel, bootloader, Wic, manifest, license,
   SPDX/SBOM, checksum, and other deploy records inside the adapter.
6. Return byte size and modification time plus typed associated checksum,
   manifest, license, SPDX/SBOM, and Wic paths. Unavailable data must remain
   unavailable; do not infer paths from build logs.
7. Parse bounded checksum files only in the adapter, validate digest records,
   and report malformed, truncated, unsupported, or unassociated data as
   explicit limitations.
8. Add a cancellable adapter request/response boundary and conversion to the
   existing typed `BackendEvent`; distinguish cancellation, timeout, empty
   inventory, partial results, and failures.
9. Add fake-filesystem/temp-directory tests named `image_artifact_adapter` for
   normal, empty, partial, malformed, oversized, symlink, path-escape,
   missing-directory, timeout/cancellation, and deterministic ordering paths.
10. Update `docs/architecture.md` for adapter ownership, then update
    registry/status and hand off to `IMAGES-UI-001`.

## Definition of done

- Configured deploy discovery is authoritative and version-compatible.
- The adapter returns bounded typed artifacts and limitations without exposing
  raw filesystem or checksum text to the model/UI.
- Cancellation, timeout, malformed input, and path safety are tested.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake image_artifact_adapter
cargo test -p yoctui-app image_artifact_adapter
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
