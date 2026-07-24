# Yoctui Implementation Status

The machine-readable source of truth is `docs/task-registry.toml`.

Status values:

- `NOT_STARTED`
- `IN_PROGRESS`
- `BLOCKED`
- `DONE`

## Current phase

Complete the live Tasks workspace. The typed backend boundary, command palette, animation, interactive settings, semantic themes, unified dialogs, responsive workbench, persistent jobs, and first live BitBake matrix are complete.

## Current task

See `docs/current-task.md`.

## Milestone summary

| Milestone | Status | Notes |
|---|---|---|
| M0 Governance | DONE | Contracts, registry, active-task handoff, and repository reconciliation are in place |
| M1 BitBake cockpit | IN_PROGRESS | Typed background build execution and live BitBake 2.19.0 Tinfoil validation exist; remaining typed cockpit workflows are incomplete |
| M2 Persistent workbench | IN_PROGRESS | Persistent shell, responsive modes, focus, dialogs, and background build jobs are complete; palette, Tasks, and preferences remain partial |
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
| Dialogs | DONE | One typed FIFO queue drives build, image, recipe, Devtool, BBMASK, editor, quit, and completion workflows; invalid actions are inert and asynchronous completion waits behind active input |
| Command palette | DONE | Typed catalog, case-insensitive search, contextual availability, disabled explanations, inert invalid activation, focus restore, themes, and narrow rendering are covered |
| Themes | DONE | Five complete semantic palettes cover shell, focus, selection, status, severity, progress, dialogs, notifications, and syntax; monochrome/no-color use terminal attributes |
| Task animation | DONE | UI-tick fast/slow cadence, stable reduced-motion activity, honest unknown progress, and nonanimated determinate/terminal rows have reducer and TestBackend coverage |
| Background-job model | DONE | Stable IDs, typed lifecycle/context/progress/result/error, bounded output/history, cancellation capability, and reducer coverage are implemented |
| Background build execution | DONE | Confirmed builds allocate one job; typed events drive lifecycle/output; navigation persists; failure, cancellation rejection/acknowledgement, and backend loss are covered |
| Live BitBake bridge | DONE | Tinfoil-backed workspace, variable, recipe, layer, parse/task/log events, normal completion, cancellation, and shutdown passed against BitBake 2.19.0 / Poky 6.0.99 snapshot on qemux86-64 |
| Typed backend boundary | DONE | Typed workspace and metadata events normalize in the app into reducer actions; unknown events are safe, missing progress remains unknown, terminal lifecycle updates are singular, and the UI boundary rejects backend parsing |
| Layers workspace | IN_PROGRESS | Lazy directory descent, parent navigation, subtree refresh, file preview, and editing exist (`a7512fa`, `c7128a6`); hidden files, Git state, and safe large-file handling remain |
| Tasks workspace | IN_PROGRESS | Active task list and persistent navigation exist (`854f798`); completed/waiting rows, filters, inspector selection, and direct tests remain |
| Images workspace | IN_PROGRESS | Image-recipe listing and confirmed image builds exist (`7fb89fb`); deploy artifacts, manifests, checksums, licenses, and inspector details remain |
| Settings workspace | DONE | Six typed visual/log rows apply immediately, persist atomically without rewriting config.toml, preserve precedence, and retain retryable dirty state on failure |
| Signature workflows | NOT_STARTED | No adapter, typed workflow, UI, or tests are present |
| Package data browser | NOT_STARTED | No `oe-pkgdata-util` adapter, workspace, typed workflow, or tests are present |

## Priority queue

1. `TASKS-001` — complete live Tasks workspace
2. `LOG-001` — complete bounded searchable logs
3. `ERROR-001` — complete error workspace
4. `LAYERS-001` — complete lazy layer tree and inspector
5. `RECIPES-001` — complete recipe actions and inspector
6. `CONFIG-001` — complete configuration provenance and editing
7. `DEVTOOL-001` — complete Devtool lifecycle

## Rules

- This document must agree with `docs/task-registry.toml`.
- Parent capability descriptions are not completion evidence.
- A task is `DONE` only after its verification command passes.
- Every intentional UI change updates `docs/ui-spec.md`.
- Every architecture change updates `docs/architecture.md`.
- Completed tasks should include the implementing commit in the registry notes.
