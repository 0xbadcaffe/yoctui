# Current task

## Active task

**ID:** GOV-001  
**Title:** Reconcile the governance roadmap with the current repository

## Objective

Make the task registry and human-readable status accurately reflect the code currently present on `master`.

## Required work

1. Inspect recent commits and the implementation of:
   - persistent Header / Navigator / Workspace / Inspector / Footer shell
   - focus routing and modal focus trapping
   - command palette
   - themes
   - task animations and reduced motion
   - lazy Layers tree behavior
   - Tasks workspace
   - Images workspace
   - Settings workspace
2. Update `docs/task-registry.toml` statuses only when supported by code and tests.
3. Update `docs/implementation-status.md` to match the registry.
4. Do not implement unrelated product features in this task.

## Definition of done

- No registry task is marked `DONE` without code and verification evidence.
- Recent UI work is no longer incorrectly marked `NOT_STARTED`.
- `docs/implementation-status.md` and `docs/task-registry.toml` agree.
- `./scripts/verify-roadmap.sh` passes.
- Relevant existing tests pass.
- One coherent commit is created.

## Verification

```bash
./scripts/verify-roadmap.sh
cargo test -p yoctui-model -p yoctui-ui -p yoctui-app
```

## Next task

Select the highest-priority eligible task from `docs/task-registry.toml`.

Recommended next candidate after reconciliation:

```text
JOB-001 — Add the persistent background-job domain model
```
