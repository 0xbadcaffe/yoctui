# Current Task

## Task

**ID:** MAINT-RELEASE-ADAPTER-001
**Title:** Adapt locked signatures, comparisons, and archives

## Objective

Discover, preview, and execute the exact installed locked-signature cache,
build-history comparison, optional build-compare, and Git archive interfaces
with canonical identities, bounded evidence, cancellation, timeout, and
runner-loss handling.

## Required work

1. Inspect an explicit initialized build snapshot and child-only search path
   for canonical regular non-symlink `gen-lockedsig-cache`,
   `buildhistory-diff`, optional `build-compare`, and `oe-git-archive` tools.
   Missing tools and distinct interfaces remain explicit.
2. Construct the exact ordered `gen-lockedsig-cache` vector from a canonical
   readable locked-signature include, canonical input/output cache roots,
   native LSB string, and optional canonical readable filter. Preview output
   replacement risk and revalidate every identity before execution.
3. Validate one canonical build-history Git repository plus zero, one, or two
   revisions. Construct only documented `buildhistory-diff` flags for report
   version/all, signatures, signature differences, exclusions, and no-colour.
4. Keep optional `build-compare` capability and vectors distinct from
   `buildhistory-diff`; never emulate one by relabelling the other.
5. Construct exact `oe-git-archive` vectors from canonical data/repository
   roots and typed create/bare/tag/branch/message/exclusion/note choices.
   Retain local archive creation separately from an optional remote push.
6. Revalidate executable, Git revision, input, output, and evidence identities
   immediately before execution or installation. Successful replaceable
   evidence is installed atomically; failures retain prior valid evidence.
7. Reuse the shared Maintenance process-group runner with bounded streams and
   fake filesystem/process tests for exact vectors, missing/unsafe/tampered
   inputs, success, nonzero failure, timeout, graceful/forced cancellation,
   rejection, and runner loss.
8. Do not claim live signature-cache, comparison, archive, or network
   compatibility from fixture tests.

## Definition of done

- Every supported release operation is capability-driven, shell-free, and
  exact; missing optional tools remain unavailable.
- Replacement/destructive and network side effects are explicit in previews.
- All inputs and resulting evidence are bounded and revalidated.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake maintenance_release
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/architecture.md` only if the adapter boundary changes.
- Mark `MAINT-RELEASE-ADAPTER-001` `DONE` only after verification passes.
- Update `docs/implementation-status.md`.
- Replace this file with the next eligible highest-priority Maintenance task.

## Next task

`MAINT-OPTIONAL-ADAPTER-001`
