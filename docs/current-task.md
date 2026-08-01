# Current Task

## Task

**ID:** MAINT-SSTATE-UI-001
**Title:** Add typed sstate readiness and cleanup forms

## Objective

Implement the missing model-owned, focus-trapped `c` readiness and `d`
protected-cleanup entry workflows specified for the Maintenance Sstate view,
without executing a process in this task.

## Required work

1. Inspect existing Maintenance requests, previews, generic confirmations,
   renderer, and input helpers; reuse them rather than duplicating execution.
2. Add bounded typed readiness draft state for targets, mode, output/log paths,
   and timeout, initialized only from authoritative capability metadata.
3. Add bounded typed cleanup draft state for cache root, stamps roots, modes,
   and worker count. Candidate discovery remains a typed effect for the CLI task;
   do not fabricate candidates in the model or UI.
4. Map `c` and `d` only in the Sstate view. Dialog input must trap focus,
   validate visibly, and close without side effects on `Esc`.
5. Render every field, validation/disabled reason, and footer shortcut safely at
   wide, medium, narrow, too-small, theme, and no-color boundaries.
6. Add reducer, app mapping, and Ratatui TestBackend tests for normal, invalid,
   unavailable, and cancellation paths.

## Definition of done

- The specified readiness and cleanup forms are reachable and model-owned.
- Confirming a valid form emits only a typed preview-acquisition effect.
- No command runs and no cleanup candidate is inferred in this task.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model maintenance_sstate_workspace
cargo test -p yoctui-app maintenance_sstate_workspace
cargo test -p yoctui-ui maintenance_sstate_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` only if intentional behavior differs from the current
  authoritative Sstate contract.
- Mark `MAINT-SSTATE-UI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-SSTATE-CLI-001`.

## Next task

`MAINT-SSTATE-CLI-001`
