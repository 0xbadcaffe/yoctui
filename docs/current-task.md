# Current task

## Active task

**ID:** QA-MODEL-001
**Title:** Close typed QA model gate

## Objective

Audit and close the shared pure model/app gate for the complete Recipe &
Kernel and Layer QA workspace without adding adapter, process, CLI, or final
rendering behavior.

## Required work

1. Inspect the complete QA model, app input mapping, authoritative QA
   specification, architecture boundary, and both completed child tasks.
2. Confirm that recipe/provider and configured-layer scopes, capability
   states/catalog entries, operation/session/report/finding identities,
   previews, bounds, search/filter/drill state, dialogs, exact opens, and
   terminal outcomes agree across both views.
3. Confirm recipe/kernel managed-build cancellation and native layer-runner
   cancellation remain identity-correlated and cannot target one another.
4. Add only missing cross-view reducer or mechanical app tests discovered by
   the audit; do not duplicate child behavior or begin adapter work.
5. Run the complete focused model/app gate and every baseline verification
   command.
6. Mark the parent task done only after all focused and baseline verification
   passes, then select the next highest-priority eligible adapter task.

## Definition of done

- The shared typed QA state matches the UI and architecture contracts.
- Both child task gates and the combined `qa_`/`qa_workflow` gates pass.
- No model path guesses tasks, tools, providers, configured layers, reports,
  findings, shell syntax, or cancellation targets.
- Roadmap state advances to the first eligible QA adapter task.

## Verification

```bash
cargo test -p yoctui-model qa_
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
