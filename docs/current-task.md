# Current task

## Active task

**ID:** QA-LAYER-ADAPTER-001
**Title:** Run exact layer QA checks

## Objective

Discover and revalidate canonical `yocto-check-layer` capability for exact
configured layers, then execute only capability-supplied shell-free vectors
with bounded typed lifecycle events.

## Required work

1. Inspect the typed layer-QA model, configured-layer metadata, hardened native
   runner adapters, process-group cancellation patterns, and the authoritative
   QA architecture before writing code.
2. Add a focused BitBake layer-QA adapter that accepts an initialized build
   identity, a child-only executable search path, and exact configured-layer
   identities; do not reconstruct layer roots from names or scan arbitrary
   directories.
3. Discover a canonical regular executable without following symlinks and
   construct an exact capability vector for each configured layer. Preserve
   missing/unsafe tools and invalid optional layers as typed disabled or
   partial capability with stable reasons.
4. Capture executable size/modification identity and canonical layer roots,
   reconstruct only the confirmed indexed vector, and revalidate both
   immediately before spawn.
5. Launch one native-argv child in its own process group, use no shell or
   free-form arguments, bound and tag both output streams, reject duplicate
   sessions, and emit exact started/output/completed/nonzero events.
6. Implement correlated graceful cancellation with forced escalation and
   distinct rejection, timeout, stale identity, and unexpected worker/channel
   loss outcomes.
7. Add fake-process and fake-filesystem tests for discovery, exact vectors,
   symlink/escape/tampering, bounded output, duplicate start, success,
   nonzero, graceful/forced cancellation, timeout, and loss. Add only
   mechanical app event mapping; do not implement CLI polling or rendering.

## Definition of done

- Capability contains only canonical configured-layer and executable
  identities with complete indexed native vectors.
- Spawn reconstructs the confirmed vector exactly after immediate identity
  revalidation and never invokes a shell.
- Every lifecycle, cancellation, timeout, duplicate, stale, and loss outcome
  remains typed and distinct.
- Focused adapter/app and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake qa_layer
cargo test -p yoctui-app qa_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
