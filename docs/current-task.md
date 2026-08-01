# Current Task

## Task

**ID:** MAINT-UI-CLI-001
**Title:** Integrate complete Maintenance workspace

## Objective

Audit and verify the complete typed Maintenance workspace across model, app,
adapter, Ratatui, and CLI boundaries now that every atomic operation-entry and
execution task is complete.

## Required work

1. Inspect the committed Sstate, Services, Release, and optional-integration
   paths against `docs/ui-spec.md` and `docs/architecture.md`; do not duplicate
   existing behavior.
2. Verify every specified entry action reaches a typed form or typed operation,
   every execution route uses authoritative adapter previews, and no widget
   parses process output or starts a process directly.
3. Verify exact correlation, focus trapping, responsive rendering, navigation,
   cancellation, bounded evidence/output, and terminal-state behavior across
   the combined Maintenance test matrix.
4. Reconcile any cross-layer defect with the smallest applicable tests and
   documentation update. Preserve explicit limitations and do not claim live
   compatibility from fixture tests.

## Definition of done

- The complete Maintenance workflow matches the authoritative UI and
  architecture contracts across every crate.
- Combined model, app, adapter, TestBackend, and CLI verification passes.
- The baseline verification suite passes.
- `MAINT-UI-CLI-001` is marked `DONE` and `MAINT-001` becomes the single active
  task.

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

- Update `docs/ui-spec.md` only for an intentional UI behavior correction.
- Update `docs/architecture.md` only for an architecture correction.
- Mark `MAINT-UI-CLI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-001`.

## Next task

`MAINT-001`
