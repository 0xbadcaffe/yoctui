# Current task

## Active task

**ID:** QA-CHECK-MODEL-001
**Title:** Model typed recipe and kernel QA checks

## Objective

Implement pure typed state and reducer behavior for recipe and kernel QA
checks without filesystem, process, report-text, or UI parsing.

## Required work

1. Inspect the model's existing recipe identities, build requests, background
   jobs, dialogs, selection/filter patterns, and the authoritative QA
   specification before adding state.
2. Add a QA module with exact recipe/provider scope and stable typed check,
   operation, session, report, and finding identities for kernel
   configuration, URI, patch, license, and recipe/package checks.
3. Model capability not-inspected/loading/available/partial/failed states and
   a capability-supplied exact check catalog with explicit availability
   reasons; never infer tasks from check family or release.
4. Model deterministic indexed managed-BitBake previews, confirmation,
   background-job attachment, bounded session history/output summary,
   cancellation/rejection, and success/failure/cancel/timeout/loss outcomes.
5. Model generation-correlated report inventories with not-loaded/loading,
   available-empty/available/partial/failure/cancel/timeout/loss states,
   bounded normalized findings/metadata/limitations, and stale-result
   rejection.
6. Add typed search/status filters, stable selection and finding drill,
   import/refresh, exact report/provider/source open effects, and
   focus-trapping operation/import/cancellation dialogs.
7. Add reducer/unit tests for valid and invalid identities, catalog
   availability, current/stale confirmations, lifecycle transitions, bounds,
   empty/partial/failure states, selection/search/filter stability, navigation,
   and exact effects.
8. Add mechanical app key/dialog mapping tests only as required by this state;
   do not implement layer-QA state in this task.

## Definition of done

- Recipe/kernel QA state is pure, bounded, identity-correlated, and
  capability-driven.
- Exact previews/effects never contain guessed tasks, paths, or shell text.
- Dialog focus and every lifecycle/report state are explicit.
- Focused model/app and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model qa_check_workflow
cargo test -p yoctui-app qa_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
