# Yoctui Implementation Status

The machine-readable source of truth is `docs/task-registry.toml`.

Status values:

- `NOT_STARTED`
- `IN_PROGRESS`
- `BLOCKED`
- `DONE`

## Current phase

Persistent background jobs and live BitBake reliability. The repository reconciliation is complete.

## Current task

See `docs/current-task.md`.

## Milestone summary

| Milestone | Status | Notes |
|---|---|---|
| M0 Governance | DONE | Contracts, registry, active-task handoff, and repository reconciliation are in place |
| M1 BitBake cockpit | IN_PROGRESS | Strong mocked/process foundation; live matrix incomplete |
| M2 Persistent workbench | IN_PROGRESS | Persistent shell and background-job domain model are complete; execution, responsive modes, palette, Tasks, preferences, and dialogs remain partial |
| M3 Development workbench | IN_PROGRESS | Recipes, lazy layer browsing, config provenance, and Devtool are partial; signatures and package data have not started |
| M4 Images/SDK/QEMU/Wic | IN_PROGRESS | Image-recipe listing and build selection exist; artifact, SDK, QEMU, and Wic workflows remain |
| M5 Testing/QA/Security | NOT_STARTED | Coverage infrastructure exists; product workflows remain |
| M6 Maintenance | NOT_STARTED | Partial diagnostics only |
| M7 Hardening | IN_PROGRESS | Coverage and profiling foundations exist |

## Reconciliation evidence

| Capability | Status | Evidence and remaining work |
|---|---|---|
| Persistent application shell | DONE | Header, Navigator, Workspace, Inspector, and Footer remain visible during builds (`fc1b1ae`, `4db7369`); breakpoint TestBackend coverage is in `88b4aa7` |
| Responsive layouts | IN_PROGRESS | Wide, medium, narrow, and too-small dimensions render safely; medium inspector overlay and narrow pane switcher remain |
| Focus and dialogs | IN_PROGRESS | Focus cycling and modal key trapping exist (`4f0d7eb`, `5e4bbd9`); dialogs are still separate App fields without unified focus restoration |
| Command palette | IN_PROGRESS | Ctrl+P overlay, selection, and activation exist (`457f176`); search, contextual availability, explanations, and direct tests remain |
| Themes | IN_PROGRESS | Five built-in names and focus/selection roles exist (`88816bd`); complete semantic roles and interactive persistence remain |
| Task animation | IN_PROGRESS | Tick-driven fast/slow indeterminate frames and reduced-motion suppression exist (`3f69c16`); direct behavior tests remain |
| Background-job model | DONE | Stable IDs, typed lifecycle/context/progress/result/error, bounded output/history, cancellation capability, and reducer coverage are implemented |
| Layers workspace | IN_PROGRESS | Lazy directory descent, parent navigation, subtree refresh, file preview, and editing exist (`a7512fa`, `c7128a6`); hidden files, Git state, and safe large-file handling remain |
| Tasks workspace | IN_PROGRESS | Active task list and persistent navigation exist (`854f798`); completed/waiting rows, filters, inspector selection, and direct tests remain |
| Images workspace | IN_PROGRESS | Image-recipe listing and confirmed image builds exist (`7fb89fb`); deploy artifacts, manifests, checksums, licenses, and inspector details remain |
| Settings workspace | IN_PROGRESS | Current theme/animation preferences render (`32b7983`); interactive editing and complete persistence remain |
| Signature workflows | NOT_STARTED | No adapter, typed workflow, UI, or tests are present |
| Package data browser | NOT_STARTED | No `oe-pkgdata-util` adapter, workspace, typed workflow, or tests are present |

## Priority queue

1. `JOB-002` — job effect execution and cancellation
2. `BB-001` — real BitBake smoke harness
3. `UI-RESP-001` — complete responsive shell matrix
4. `FOCUS-001` — complete shared focus routing
5. `DIALOG-001` — unified typed dialog stack
6. `PALETTE-001` — searchable contextual command palette
7. `TASKS-001` — complete live Tasks workspace
8. `SETTINGS-001` — interactive settings and persistence
9. `LAYERS-001` — complete lazy layer tree and inspector
10. `IMAGES-001` — complete image artifact workspace
11. `DEP-001` — dependency exploration
12. `QEMU-001` — managed QEMU workflow

## Rules

- This document must agree with `docs/task-registry.toml`.
- Parent capability descriptions are not completion evidence.
- A task is `DONE` only after its verification command passes.
- Every intentional UI change updates `docs/ui-spec.md`.
- Every architecture change updates `docs/architecture.md`.
- Completed tasks should include the implementing commit in the registry notes.
