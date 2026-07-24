# Current task

## Active task

**ID:** BB-002
**Title:** Complete the typed backend-to-model event boundary

## Objective

Enforce one complete typed boundary from BitBake/process/bridge adapters through
application event mapping into reducer actions, with no raw backend parsing or
state mutation in UI widgets.

## Required work

1. Inventory every backend event variant, protocol event, app mapping branch,
   reducer action, and widget consumer before changing the boundary.
2. Ensure protocol and backend events carry typed workspace, recipe, layer,
   variable, dependency, relationship, parse, task, log, completion,
   cancellation, command-failure, and disconnect data.
3. Centralize backend-event-to-model-action normalization in `yoctui-app`.
4. Remove any raw backend/process/protocol parsing from CLI orchestration,
   model reducers, and Ratatui widgets.
5. Preserve unknown/new protocol-event safety without inventing model state.
6. Ensure malformed, oversized, out-of-order, and disconnected input produces
   typed failure/loss behavior rather than panics or silent state mutation.
7. Verify build-job lifecycle actions and primary model actions are both
   emitted exactly once for every relevant backend terminal event.
8. Add typed-event tests in protocol, bitbake, app, model, and UI boundary
   enforcement tests as applicable.
9. Update `docs/architecture.md` if normalization ownership needs
   clarification.

## Definition of done

- Every backend event reaches the model through a typed app mapping.
- Terminal events update both build state and the persistent job exactly once.
- UI widgets consume typed model state and never parse backend text.
- Unknown/malformed/disconnected input is safe and observable.
- Boundary enforcement and typed-event tests pass.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
./scripts/verify-ui-spec.sh
cargo test -p yoctui-protocol typed_event
cargo test -p yoctui-bitbake typed_event
cargo test -p yoctui-app typed_event
cargo test -p yoctui-model typed_event
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`TASKS-001 — Complete the live Tasks workspace`
