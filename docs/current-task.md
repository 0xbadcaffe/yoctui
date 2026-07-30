# Current task

## Active task

**ID:** QA-CLI-001
**Title:** Integrate non-blocking QA execution

## Objective

Wire the complete typed QA model, adapters, managed BitBake coordinator,
report worker, native layer runner, exact opens, and terminal input into the
CLI without blocking navigation or unrelated operations.

## Required work

1. Inspect existing Testing/Security CLI coordinators and QA effects before
   changing code.
2. Inspect recipe/kernel and layer capabilities on demand from authoritative
   workspace metadata and child-only executable search state.
3. Reuse the managed BitBake coordinator for recipe/kernel operations and
   correlate completion/cancellation to the exact QA session.
4. Own one replaceable generation-correlated report worker and one independent
   layer-QA runner; poll both without blocking terminal input or navigation.
5. Route imports, refreshes, exact report/provider/source/layer opens through
   immediate adapter revalidation.
6. Preserve duplicate, rejection, nonzero, empty, partial, failure,
   cancellation, timeout, stale, and loss outcomes as typed actions.
7. Map every specified QA key and modal key without leaking dialog input.
8. Add CLI integration tests for discovery, managed/native execution,
   reports, cancellation, exact opens, terminal outcomes, and navigation.

## Definition of done

- Both QA workflows execute and poll without blocking the TUI.
- Managed and native cancellation targets remain independent.
- Exact identities are revalidated before spawn/open.
- Focused CLI and baseline verification pass.

## Verification

```bash
cargo test -p yoctui -- qa_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
