# Yoctui Implementation Status

The machine-readable source of truth is `docs/task-registry.toml`.

Status values:

- `NOT_STARTED`
- `IN_PROGRESS`
- `BLOCKED`
- `DONE`

## Current phase

Governance reconciliation, persistent background jobs, and live BitBake reliability.

## Current task

See `docs/current-task.md`.

## Milestone summary

| Milestone | Status | Notes |
|---|---|---|
| M0 Governance | IN_PROGRESS | Package introduced; repository reconciliation required |
| M1 BitBake cockpit | IN_PROGRESS | Strong mocked/process foundation; live matrix incomplete |
| M2 Persistent workbench | IN_PROGRESS | Shell foundations exist; jobs/preferences/dialogs incomplete |
| M3 Development workbench | IN_PROGRESS | Recipes, layers, config, Devtool partially implemented |
| M4 Images/SDK/QEMU/Wic | NOT_STARTED | Some Images UI foundation exists |
| M5 Testing/QA/Security | NOT_STARTED | Coverage infrastructure exists; product workflows remain |
| M6 Maintenance | NOT_STARTED | Partial diagnostics only |
| M7 Hardening | IN_PROGRESS | Coverage and profiling foundations exist |

## Priority queue

1. `GOV-001` — reconcile registry with current code
2. `JOB-001` — persistent background-job domain model
3. `JOB-002` — job effect execution and cancellation
4. `BB-001` — real BitBake smoke harness
5. `UI-RESP-001` — complete responsive shell matrix
6. `DIALOG-001` — unified typed dialog stack
7. `SETTINGS-001` — interactive settings and persistence
8. `LAYERS-001` — complete lazy layer tree and inspector
9. `DEP-001` — dependency exploration
10. `QEMU-001` — managed QEMU workflow

## Rules

- This document must agree with `docs/task-registry.toml`.
- Parent capability descriptions are not completion evidence.
- A task is `DONE` only after its verification command passes.
- Every intentional UI change updates `docs/ui-spec.md`.
- Every architecture change updates `docs/architecture.md`.
- Completed tasks should include the implementing commit in the registry notes.
