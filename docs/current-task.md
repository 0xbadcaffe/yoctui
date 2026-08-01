# Current Task

## Task

**ID:** MAINT-RELEASE-UI-001
**Title:** Close typed Maintenance release form gate

## Objective

Audit and verify the locked-cache, build-history, and Git archive forms as one
coherent Release view before enabling execution.

## Required work

1. Inspect the three committed form paths against `docs/ui-spec.md` and the
   adapter-owned request types; reconcile any disagreement without adding a new
   operation or layout.
2. Verify `l/h/a` are view-scoped, capability-gated, focus-trapped, bounded,
   and side-effect free before their typed preview effects.
3. Verify authoritative context, every documented control/default, validation,
   destructive/replacement meaning, local-versus-network intent, and responsive
   80x24 rendering agree across model, app, and UI.
4. Run the combined focused matrix and full baseline. Do not mark this gate
   complete from individual form tests alone and do not claim live tool
   compatibility.

## Definition of done

- All three forms and their exact UI contract agree across layers.
- Combined focused and baseline verification pass without zero-test filters.
- No preview effect is executed before the next CLI task.

## Verification

```bash
cargo test -p yoctui-model maintenance_release_
cargo test -p yoctui-app maintenance_release_
cargo test -p yoctui-ui maintenance_release_
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` only if the audit finds a behavioral disagreement.
- Update `docs/architecture.md` only if component ownership changes.
- Mark `MAINT-RELEASE-UI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-RELEASE-CLI-001`.

## Next task

`MAINT-RELEASE-CLI-001`
