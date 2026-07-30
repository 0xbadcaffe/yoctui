# Current task

## Active task

**ID:** QA-LAYER-MODEL-001
**Title:** Model typed layer QA checks

## Objective

Extend the pure QA state and reducer with exact configured-layer selection and
one typed `yocto-check-layer` workflow without filesystem, process, native
output, or UI parsing.

## Required work

1. Inspect the new QA recipe/kernel model, configured layer identities, native
   runner models, dialogs, selection/filter behavior, and the authoritative QA
   specification before adding state.
2. Add exact configured-layer name/root identities and capability
   not-inspected/loading/available/partial/failed states with a canonical
   executable and capability-supplied shell-free argument vector.
3. Keep every configured layer visible when the tool is unavailable and expose
   stable disabled reasons; never scan arbitrary roots or derive a path from a
   display name.
4. Model deterministic indexed previews, immediate identity revalidation
   effects, one independent active native session, bounded tagged output and
   history, typed result counts/findings, duplicate rejection, cancellation
   rejection, and success/nonzero/cancel/timeout/loss outcomes.
5. Add exact configured-layer selection, search/status filtering, result drill,
   exact report/provider/source opens where applicable, and focus-trapping
   operation/cancellation dialogs without weakening the recipe/kernel flow.
6. Add reducer/unit tests for invalid and stale layer/tool identities,
   unavailable/partial capability, deterministic vectors, lifecycle and
   bounds, selection/navigation, typed findings, every terminal outcome, and
   exact effects.
7. Extend mechanical app key/dialog mapping tests only as required by the
   layer view; do not add adapters, CLI process execution, or final rendering
   in this task.

## Definition of done

- Configured-layer QA state is pure, bounded, identity-correlated, and
  capability-driven.
- Exact previews/effects contain only adapter-supplied executable, arguments,
  and configured-layer identities.
- Recipe/kernel and layer state coexist without sharing active-runner
  cancellation targets.
- Focused model/app and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model qa_layer_workflow
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
