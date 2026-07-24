# Yoctui Implementation Status

The machine-readable source of truth is `docs/task-registry.toml`.

Status values:

- `NOT_STARTED`
- `IN_PROGRESS`
- `BLOCKED`
- `DONE`

## Current phase

Unified typed dialog state. Shared focus routing, responsive layouts, persistent jobs, and the first live BitBake compatibility matrix are complete.

## Current task

See `docs/current-task.md`.

## Milestone summary

| Milestone | Status | Notes |
|---|---|---|
| M0 Governance | DONE | Contracts, registry, active-task handoff, and repository reconciliation are in place |
| M1 BitBake cockpit | IN_PROGRESS | Typed background build execution and live BitBake 2.19.0 Tinfoil validation exist; remaining typed cockpit workflows are incomplete |
| M2 Persistent workbench | IN_PROGRESS | Persistent shell and background build jobs are complete; responsive modes, palette, Tasks, preferences, and dialogs remain partial |
| M3 Development workbench | IN_PROGRESS | Recipes, lazy layer browsing, config provenance, and Devtool are partial; signatures and package data have not started |
| M4 Images/SDK/QEMU/Wic | IN_PROGRESS | Image-recipe listing and build selection exist; artifact, SDK, QEMU, and Wic workflows remain |
| M5 Testing/QA/Security | NOT_STARTED | Coverage infrastructure exists; product workflows remain |
| M6 Maintenance | NOT_STARTED | Partial diagnostics only |
| M7 Hardening | IN_PROGRESS | Coverage and profiling foundations exist |

## Reconciliation evidence

| Capability | Status | Evidence and remaining work |
|---|---|---|
| Persistent application shell | DONE | Header, Navigator, Workspace, Inspector, and Footer remain visible during builds (`fc1b1ae`, `4db7369`); breakpoint TestBackend coverage is in `88b4aa7` |
| Responsive layouts | DONE | Wide three-pane mode, medium Inspector overlay, narrow visible pane switcher, too-small messaging, resize preservation, and all-screen boundary tests are complete |
| Focus routing | DONE | Bidirectional pane cycling, modal input trapping, nested-modal return targets, exact pane restoration, quit cancellation, and responsive focus rendering are covered |
| Dialogs | IN_PROGRESS | Build, image, recipe, Devtool, BBMASK, notification, and completion overlays exist; dialog state remains split across ad-hoc App fields |
| Command palette | IN_PROGRESS | Ctrl+P overlay, selection, and activation exist (`457f176`); search, contextual availability, explanations, and direct tests remain |
| Themes | IN_PROGRESS | Five built-in names and focus/selection roles exist (`88816bd`); complete semantic roles and interactive persistence remain |
| Task animation | IN_PROGRESS | Tick-driven fast/slow indeterminate frames and reduced-motion suppression exist (`3f69c16`); direct behavior tests remain |
| Background-job model | DONE | Stable IDs, typed lifecycle/context/progress/result/error, bounded output/history, cancellation capability, and reducer coverage are implemented |
| Background build execution | DONE | Confirmed builds allocate one job; typed events drive lifecycle/output; navigation persists; failure, cancellation rejection/acknowledgement, and backend loss are covered |
| Live BitBake bridge | DONE | Tinfoil-backed workspace, variable, recipe, layer, parse/task/log events, normal completion, cancellation, and shutdown passed against BitBake 2.19.0 / Poky 6.0.99 snapshot on qemux86-64 |
| Layers workspace | IN_PROGRESS | Lazy directory descent, parent navigation, subtree refresh, file preview, and editing exist (`a7512fa`, `c7128a6`); hidden files, Git state, and safe large-file handling remain |
| Tasks workspace | IN_PROGRESS | Active task list and persistent navigation exist (`854f798`); completed/waiting rows, filters, inspector selection, and direct tests remain |
| Images workspace | IN_PROGRESS | Image-recipe listing and confirmed image builds exist (`7fb89fb`); deploy artifacts, manifests, checksums, licenses, and inspector details remain |
| Settings workspace | IN_PROGRESS | Current theme/animation preferences render (`32b7983`); interactive editing and complete persistence remain |
| Signature workflows | NOT_STARTED | No adapter, typed workflow, UI, or tests are present |
| Package data browser | NOT_STARTED | No `oe-pkgdata-util` adapter, workspace, typed workflow, or tests are present |

## Priority queue

1. `DIALOG-001` — unified typed dialog stack
2. `PALETTE-001` — searchable contextual command palette
3. `TASKS-001` — complete live Tasks workspace
4. `SETTINGS-001` — interactive settings and persistence
5. `LAYERS-001` — complete lazy layer tree and inspector
6. `IMAGES-001` — complete image artifact workspace
7. `DEP-001` — dependency exploration
8. `QEMU-001` — managed QEMU workflow

## Rules

- This document must agree with `docs/task-registry.toml`.
- Parent capability descriptions are not completion evidence.
- A task is `DONE` only after its verification command passes.
- Every intentional UI change updates `docs/ui-spec.md`.
- Every architecture change updates `docs/architecture.md`.
- Completed tasks should include the implementing commit in the registry notes.
