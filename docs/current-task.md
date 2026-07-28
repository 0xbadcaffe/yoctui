# Current task

## Active task

**ID:** SDK-ARTIFACT-001
**Title:** Discover authoritative SDK artifacts

## Objective

Implement the bounded filesystem adapter that scans only the typed canonical
SDK deploy root and returns exact typed SDK artifact identities, associations,
metadata availability, and explicit limitations.

## Required work

1. Inspect the completed SDK model identities/inventory contract and existing
   image/Wic/package filesystem adapters before writing code.
2. Add an SDK artifact adapter in `yoctui-bitbake` that validates the request,
   canonicalizes the authoritative `SDK_DEPLOY` root, rejects root mismatch,
   symlinks, escapes, non-regular files, malformed names, and over-limit input,
   and never follows links.
3. Classify installers, checksum files, manifests, and other SDK records in the
   adapter only. Preserve byte size and modification time; expose machine,
   standard/extensible kind, host tuple, target tuple, and publication state
   only when authoritative, otherwise leave them unavailable.
4. Associate checksum and manifest paths deterministically without parsing in
   model, app, or widgets. Bound directories, records, path bytes,
   associations, and limitations; report skipped validly bounded records as
   partial rather than silently dropping them.
5. Provide an asynchronous cancellable entry point suitable for independent
   CLI polling. Cancellation, worker loss, missing root, permissions, empty,
   partial, and success must remain distinct.
6. Add focused fake-filesystem tests named `sdk_artifact` covering sorting,
   associations, empty, partial, malformed/oversized records, symlink/root
   escape, timeout/cancellation, unavailable metadata, and deterministic
   bounds. Do not claim live SDK compatibility.
7. Update `docs/architecture.md` only if the implemented boundary differs from
   the current contract.
8. Run focused and baseline checks, then hand off to `SDK-TOOLS-001`.

## Definition of done

- Only canonical regular records beneath the exact SDK deploy root cross the
  adapter boundary.
- Every returned identity and association validates against the model.
- Empty, partial, failed, cancelled, and successful outcomes are explicit.
- No widget/app code parses filenames or filesystem output.
- Focused adapter and all baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake sdk_artifact
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
