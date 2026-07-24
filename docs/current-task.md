# Current task

## Active task

**ID:** RECIPE-QA-001
**Title:** Add recipe CVE and SPDX background actions

## Objective

Add capability-aware CVE checking and SPDX generation for the selected recipe
using typed confirmation and the persistent background-job model.

## Required work

1. Inventory the background-job model, build-task execution path, recipe task
   metadata, current CVE/SPDX behavior, CLI routing, and retained output before
   adding a new workflow.
2. Add typed selected-recipe actions for BitBake `cve_check` and
   `create-spdx`, with availability derived from authoritative task metadata
   and an exact disabled reason when metadata or the task is unavailable.
3. Require a focus-trapping confirmation that previews the selected recipe and
   exact QA task without accepting raw command text.
4. Queue confirmed work as persistent cancellable background jobs. Preserve
   workspace navigation, typed progress/output, terminal success, failure,
   cancellation, and backend-loss state.
5. Prevent duplicate active builds/jobs and reject empty, stale, or malformed
   recipe/task state with actionable notifications.
6. Expose the CVE/SPDX shortcuts, availability, job state, and retained result
   path or absence honestly in responsive Recipes views.
7. Cover reducer, app input, confirmation, job lifecycle, CLI routing,
   unavailable/failure/cancellation paths, and Ratatui responsive rendering
   with tests named `recipe_qa_action`.

## Definition of done

- CVE and SPDX routes use only the absolute selected recipe and authoritative
  task capability.
- Confirmation previews the exact typed task and cannot interpolate raw input.
- Confirmed work remains observable and cancellable as a persistent job while
  navigation continues.
- Unavailable, duplicate, failed, cancelled, and backend-loss outcomes are
  explicit; result paths are never fabricated.
- Responsive Recipes views expose shortcuts and exact disabled reasons.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model recipe_qa_action
cargo test -p yoctui-app recipe_qa_action
cargo test -p yoctui-ui recipe_qa_action
cargo test -p yoctui -- recipe_qa_action
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-001 — Configuration provenance workspace`
