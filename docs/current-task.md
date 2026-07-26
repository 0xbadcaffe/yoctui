# Current task

## Active task

**ID:** SIG-ADAPTER-001
**Title:** Acquire and compare authoritative BitBake signatures

## Objective

Implement shell-free, bounded adapters that acquire exact BitBake recipe/task
signature records and compare selected authoritative signature files, then emit
only typed model data with honest partial and failure outcomes.

## Required work

1. Inventory existing process runners, cancellation and output bounds, BitBake
   command construction, configured build-directory handling, background jobs,
   and typed signature event hooks before adding code.
2. Inspect the locally available BitBake tools and current build metadata.
   Record exact versions and commands used for live validation; do not infer
   compatibility from mocked tests.
3. Define shell-free command plans for signature generation/dump and comparison
   using exact recipe, task, and authoritative absolute signature paths.
   Reject invalid identifiers, relative paths, and paths outside the configured
   build directory before process launch.
4. Parse bounded `bitbake-dumpsig` and `bitbake-diffsigs` results in the adapter
   into typed records and differences. Preserve unavailable data explicitly,
   normalize duplicates deterministically, and never pass raw output to the
   reducer or widgets.
5. Correlate every result and failure to its exact dump target or comparison
   request. Report truncation, malformed records, missing tools/files,
   non-zero exits, cancellation, and unsupported tool behavior honestly.
6. Integrate the adapter through typed backend events and app coordination
   without blocking the UI thread or mutating model state directly.
7. Add tests named `signature_adapter` using fake processes and fixtures for
   success, empty output, malformed/duplicate/oversized output, non-zero exits,
   missing tools/files, path escape, cancellation, and exact argument
   construction.
8. Run a live read-only smoke check against the available BitBake/Yocto build.
   Record exact version, commands, observed record/difference coverage, and any
   limitations in the task registry and implementation status.
9. Update `docs/architecture.md` for the proven adapter/tool boundary.

## Definition of done

- Signature acquisition and comparison use validated shell-free process plans.
- Only bounded typed records, differences, limitations, and exact failures cross
  the backend boundary.
- Fake-process coverage proves normal and relevant failure/cancellation paths.
- A live BitBake smoke check records real tool behavior without overstating
  support.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake signature_adapter
cargo test -p yoctui-app signature_adapter
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`SIG-UI-001 — Integrate signature dump and comparison workflows`
