# Current Task

## Task

**ID:** MAINT-UI-CLI-001
**Title:** Integrate complete Maintenance workspace

## Objective

Close the cross-layer Maintenance gate by verifying that the committed model,
adapter, app, responsive renderer, and non-blocking CLI implementation agree on
typed behavior and safety boundaries without claiming live compatibility from
fixtures.

## Required work

1. Audit every Maintenance effect, action, dialog, view, adapter result, runner
   event, and CLI route across all crates; do not duplicate existing behavior.
2. Verify correlated capability, service, and optional-integration state plus
   exact operation reconstruction, cleanup rediscovery, evidence replacement,
   archive/push separation, navigation, and isolated cancellation.
3. Verify all responsive layouts, themes, no-color meaning, disabled/failure
   states, dialogs, session output, and terminal outcomes remain model-driven.
4. Add only missing focused regression coverage discovered by the audit.
5. Run the complete focused matrix and baseline. Do not make a live support
   claim unless the opt-in real-Yocto validation is actually executed.

## Definition of done

- The complete cross-layer Maintenance matrix passes.
- Documentation and registry state match the verified implementation.
- Fixture evidence is explicitly separated from live compatibility.

## Verification

```bash
cargo test -p yoctui-model maintenance_workflow
cargo test -p yoctui-app maintenance_workflow
cargo test -p yoctui-bitbake maintenance_
cargo test -p yoctui-ui maintenance_workflow
cargo test -p yoctui -- maintenance_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update authoritative documents only if the audit reveals a disagreement.
- Mark `MAINT-UI-CLI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-001`.

## Next task

`MAINT-001`
