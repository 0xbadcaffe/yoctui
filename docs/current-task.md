# Current Task

## Task

**ID:** MAINT-CLI-001
**Title:** Integrate non-blocking Maintenance execution

## Objective

Connect model-owned Maintenance effects to replaceable capability, service,
and optional-integration workers plus one exact managed operation runner while
preserving terminal responsiveness, correlation, identity revalidation,
navigation, cancellation, and honest failure state.

## Required work

1. Inspect existing CLI worker/runner patterns, startup metadata, input routing,
   app effect mapping, and all four Maintenance adapters before changing code.
2. Route capability refresh through replaceable correlated workers that acquire
   authoritative initialized-build metadata and bounded child-only tool search
   paths, then merge typed sstate, service, release, and optional capability
   results without turning missing or partial data into empty success.
3. Route service and optional-integration diagnostics into their model-owned
   correlated states. Keep endpoint/process observations typed and detection
   only; never start or stop a service or perform optional network actions.
4. Own at most one independent Maintenance operation runner. Construct commands
   only through the adapter selected by the exact typed preview, re-inspect
   capability and revalidate executable/input/candidate/evidence identities
   immediately before spawn, and reject stale or mismatched requests visibly.
5. Poll every worker and the runner without blocking terminal or BitBake input.
   Map bounded stdout/stderr, success, nonzero exit, timeout, graceful/forced
   cancellation, duplicate rejection, start failure, and runner loss exactly
   once to typed actions while navigation continues.
6. Keep managed BitBake work on the shared build coordinator. Maintenance
   cancellation targets only the exact Maintenance session and never cancels a
   build, Devtool, QA, Security, Testing, SDK, Wic, or QEMU job.
7. Install successful replaceable evidence only after exact post-run validation;
   failure retains prior valid evidence. Keep local Git archive creation and
   optional remote push as separately confirmed operations.
8. Add CLI integration tests with fake filesystem/process adapters for refresh,
   correlation, navigation, success, nonzero, timeout, cancellation, rejection,
   loss, stale identities, and evidence replacement. Do not claim live support
   from these tests.

## Definition of done

- Maintenance inspection and execution remain non-blocking and exactly
  correlated across navigation and cancellation.
- Typed adapter results cross the CLI/app boundary without UI parsing.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui -- maintenance_workflow
cargo test -p yoctui-app maintenance_workflow
cargo test -p yoctui-bitbake maintenance_
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/architecture.md` in this commit for any CLI ownership change.
- Update `docs/ui-spec.md` only for intentional UI behavior changes.
- Mark `MAINT-CLI-001` `DONE` only after verification passes.
- Update `docs/implementation-status.md`.
- Replace this file with the next eligible highest-priority Maintenance task.

## Next task

`MAINT-UI-CLI-001`
