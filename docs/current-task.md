# Current task

## Task

**ID:** MAINT-SSTATE-ADAPTER-001
**Title:** Adapt sstate readiness and protected cleanup

## Objective

Discover, preview, and run the exact installed `oe-check-sstate` and
`sstate-cache-management.py` or legacy `.sh` interfaces with canonical
identities, bounded output, protected cleanup confirmation inputs, timeout,
cancellation, and runner loss.

## Required work

1. Inspect the initialized build environment and child-only executable search
   path for canonical regular non-symlink readiness and cleanup tools.
2. Distinguish current Python and legacy shell cleanup interfaces; never alias
   or guess missing commands.
3. Construct shell-free exact readiness vectors for:
   - one or more validated targets
   - isolated or explicitly selected same-TMPDIR behavior
   - optional exact output/log paths
   - child-only `BB_SETSCENE_ENFORCE=1`
4. Construct exact cleanup preview and execution vectors for duplicates,
   orphans, and unreferenced-by-stamps modes supported by the installed
   interface.
5. Canonicalize cache/stamps identities, reject symlinks/escapes/root paths,
   bound traversal/candidates, and revalidate executable, inputs, and the exact
   previewed candidate set before execution.
6. Implement one cancellable kill-on-drop process-group runner with bounded
   stdout/stderr, nonzero failure, timeout, graceful/forced cancellation,
   duplicate rejection, and channel/child loss.
7. Add fake-filesystem/fake-process tests for exact vectors, both interfaces,
   missing/unsafe capability, normal/empty/oversized previews, tampering,
   success, nonzero, timeout, cancellation, and loss.
8. Do not claim live sstate safety from fixture tests.

## Definition of done

- Capability distinguishes exact available, partial, unsupported, and missing
  readiness/cleanup inputs.
- No shell or unstructured argument string is used.
- Cleanup execution cannot exceed the exact confirmed canonical candidate set.
- Runner outcomes and output remain bounded and typed.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake maintenance_sstate
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/architecture.md` only if the adapter boundary changes.
- Mark `MAINT-SSTATE-ADAPTER-001` `DONE` only after verification passes.
- Update `docs/implementation-status.md`.
- Replace this file with the next eligible highest-priority Maintenance task.

## Next task

`MAINT-SERVICE-ADAPTER-001`
