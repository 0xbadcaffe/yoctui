# Yoctui Implementation Status

The machine-readable source of truth is `docs/task-registry.toml`.

Status values:

- `NOT_STARTED`
- `IN_PROGRESS`
- `BLOCKED`
- `DONE`

## Current phase

Release-quality validation is underway. The acceptance contract, real PTY
harness, and keyboard matrix are complete; the active task verifies focus and
workspace flow before visual regression coverage.

## Current task

See `docs/current-task.md`.

## Milestone summary

| Milestone | Status | Notes |
|---|---|---|
| M0 Governance | DONE | Contracts, registry, active-task handoff, and repository reconciliation are in place |
| M1 BitBake cockpit | DONE | Typed workspace discovery, builds, cancellation, events, history, telemetry, terminal restoration, and live BitBake 2.19.0 Tinfoil validation pass |
| M2 Persistent workbench | DONE | Persistent shell, responsive modes, focus, dialogs, palette, preferences, notifications, background jobs, and all specified workspaces pass their parent gates |
| M3 Development workbench | DONE | Layers, Recipes, Configuration, Devtool, dependency why-built, signatures, and the typed package-data workspace are complete |
| M4 Images/SDK/QEMU/Wic | DONE | Images, SDK, QEMU, Wic creation, and protected device writing pass their cross-layer parent gates |
| M5 Testing/QA/Security | DONE | Unified Testing, Security, and QA cross-layer parent gates pass; fake evidence remains separate from live compatibility |
| M6 Maintenance | DONE | Typed Sstate, Services, Release, and optional integrations pass their atomic, cross-layer, and milestone parent gates |
| M7 Hardening | DONE | Fuzz, stress/process-tree, ASan/LSan, property, terminal, Valgrind, deterministic profile, real perf-backed Flamegraph, CI, documentation, and completion integration pass |

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
| Logs workspace | DONE | Protected-diagnostic retention, bounded bytes/entries, safe truncation, coalescing, pressure counters, follow/pause, both-axis scrolling, search, all filters, selected Inspector, source opening, and clipboard effects are covered |
| Errors workspace | DONE | Stable structured diagnostics drive the full list and Inspector, exact retained-log and source navigation, related context, visible loss counters, and actionable success/warning/failure/cancellation/backend-loss outcomes |
| Layers workspace | DONE | Every configured layer stays visible above a stable-path lazy tree; priority, compatibility, active/Git state, subtree refresh, hidden/search filtering, typed Inspector modes, safe 64 KiB text/binary previews, and responsive failure-safe rendering are tested |
| Recipes workspace | DONE | Live-validated typed metadata, identity-stable Inspector states, typed BitBake operations, provider/log/local-patch navigation, editor failures, integrated Devtool routes, and persistent capability-aware CVE/SPDX actions are covered |
| Configuration workspace | DONE | Live-validated metadata, searchable responsive detail, typed copy/source/scope/compare actions, and allowlisted previewed atomic local.conf edits with exact refresh are complete |
| Devtool status | DONE | Absolute recipe identity, executable capability, workspace membership/source path, Git branch/head and dirty counts, typed partial/error states, shared disabled reasons, responsive rendering, fake-process tests, and a live no-workspace query are complete |
| Persistent Devtool jobs | DONE | Typed shell-free operations stream bounded stdout/stderr into durable background jobs; navigation, cancellation, all terminal outcomes, runner loss, and independent BitBake coordination are covered |
| Devtool modify/edit/build | DONE | Exact-identity status gates an explicit modify preview; successful persistent completion refreshes the authoritative source tree, opens the two-pane editor, and routes saved Ctrl+B requests through confirmed recipe builds |
| Devtool update-recipe | DONE | Exact-identity workspace eligibility gates a provider-aware confirmation; persistent success refreshes the original identity after navigation while failures retain prior status and job output |
| Devtool finish | DONE | Clean committed workspace eligibility gates an absolute configured-layer picker and exact provider/layer/path confirmation; native paths survive the adapter and completion refreshes the original identity |
| Devtool deploy-target | DONE | Exact-identity workspace eligibility and validated target drafts gate provider-aware confirmation; persistent success refreshes the original identity and failures retain status/job context |
| Devtool reset | DONE | Exact-identity removable workspace status gates provider/source destructive confirmation; persistent success refreshes expected non-membership while failures retain durable context |
| Dependency graph model | DONE | Typed recipe/task identities and build/runtime/task edges normalize deterministically; reverse lookup, cycle-safe bounded shortest why-built paths, explicit partial/failure states, selection stability, and typed app event mapping pass focused and baseline tests |
| Dependency graph acquisition | DONE | Additive typed protocol events use structured `generateDepTreeEvent`; legacy peers fall back honestly and the shell-free `bitbake -g` adapter bounds and validates task-dot output. Live BitBake 2.19.0 / Yocto 6.0.99 snapshot returned 962 nodes and 1,779 build/runtime/task edges |
| Dependency workspace | DONE | Graph-only typed rows, explicit state rendering, reverse/outgoing Inspector context, bounded why-built paths, identity-stable navigation, authoritative recipe/provider/log actions, and responsive boundary tests are complete |
| Tasks workspace | DONE | Live BitBake runqueue totals drive honest progress and aggregate waiting rows; typed active/completed/failure state, all specified filters, bounded selection, responsive tables, and contextual Inspector details are tested |
| Images workspace | DONE | Preserved recipe picker/build confirmation now coexists with bounded authoritative deploy scanning, typed artifacts/metadata, correlated cancellation, search/selection, responsive inspection, and exact build/editor actions. Live deployed-artifact compatibility is not claimed |
| Image artifact model | DONE | Exact machine/image/path identities, typed available-versus-unavailable metadata, deterministic bounds, correlated lifecycle states, identity-stable selection/search, reducer effects, and app event normalization pass focused and baseline checks |
| Image artifact adapter | DONE | Tinfoil/environment snapshots expose `DEPLOY_DIR_IMAGE`; the cancellable bounded adapter validates the machine/root, refuses symlinks and escapes, classifies deploy records, parses checksum associations, and reports partial data explicitly |
| Images artifact UI | DONE | Retained recipe picker/build confirmation now coexists with correlated CLI-owned scans, search and exact selection, typed build/editor actions, explicit lifecycle/limitations, responsive Workspace/Inspector rendering, footer hints, and direct tests |
| QEMU launch/session model | DONE | Typed capability, exact artifact-bound launch validation, deterministic preview/confirmation, stable shared-job lifecycle, bounded stream output, failures, stale events, and confirmed cancellation are covered |
| QEMU adapter | DONE | Canonical executable/artifact discovery, exact preview revalidation, shell-free native arguments, bounded stream events, success/failure/loss, duplicate rejection, and graceful/forced process-group cancellation are covered by fake processes; live compatibility is not claimed |
| QEMU dialogs/session UI | DONE | Bounded modal input, responsive capability/session rendering, and independent CLI-owned inspection/execution/polling/cancellation pass the complete cross-layer parent gate; fake runners do not establish live compatibility |
| Wic workflow | DONE | Cooked-mode creation and protected device writing pass the cross-layer gate: discovery/startup are independently polled, exact identities are revalidated immediately before spawn, responsive modal/history/telemetry state is durable, and all terminal outcomes are covered. Fake device/process coverage does not establish live Wic or removable-media compatibility |
| SDK workflow | DONE | The parent gate passes across typed model/app state, authoritative artifact and shell-free tool adapters, responsive rendering, and independent CLI execution. It covers scans, capability inspection, managed BitBake populate/test reuse, exact artifact opening, publication/native child execution, a bounded keyboard-editable native form, timeout/cancellation/loss, success refresh, navigation, and telemetry. Fake scans/processes do not claim live SDK compatibility |
| Testing workflow | DONE | The unified parent gate passes for typed launch/result state, selftest/resulttool adapters, responsive rendering, and non-blocking CLI execution. Managed BitBake reuse, independent selftest/result operations, exact correlation, navigation, cancellation, import/comparison/JUnit export, and terminal outcomes are verified. Fake-process coverage does not establish live compatibility |
| Security workflow | DONE | The complete cross-layer gate passes for capability-driven CVE checks/mapping, current/legacy recipe and image SBOM workflows, bounded exact reports, responsive UI, managed BitBake reuse, independent CLI polling, exact-open revalidation, refresh, navigation, cancellation, and explicit partial/terminal states. Focused fake evidence does not establish live Yocto compatibility |
| QA workflow | DONE | The complete parent gate passes across typed Recipe & Kernel and Layer QA state, exact capability/report/native adapters, responsive rendering, managed BitBake reuse, independent CLI polling/cancellation, replaceable reports, revalidated evidence opens, navigation, and every terminal outcome. Fixture evidence does not establish live compatibility |
| Maintenance workflow | DONE | Sstate `c/d`, Services `e/m`, Release `l/h/a`, and optional integrations pass their complete model/app/adapter/TestBackend/CLI and milestone parent gates. Execution retains exact inspection, confirmation, correlation, bounded evidence, cancellation, and terminal semantics; no live cache, PR-service, release-tool, network, or optional-tool compatibility is claimed from fixtures |
| Settings workspace | DONE | Six typed visual/log rows apply immediately, persist atomically without rewriting config.toml, preserve precedence, and retain retryable dirty state on failure |
| Signature model | DONE | Exact recipe/task/hash/path identities, explicit bounded dump/comparison states, deterministic typed differences, identity-stable selection, stale-result correlation, reducer effects, and typed backend-event mapping are verified |
| Signature adapter | DONE | Shell-free bounded dumpsig/diffsigs adapters validate canonical artifact paths, exact correlation, timeout/cancellation, typed parsing and failures. Live BitBake 2.19.0 returned two real records and 113 typed differences with one explicit recursive-detail limitation |
| Signature workspace | DONE | `Z` opens an authoritative focus-trapped task picker and responsive child workspace with exact record/sides, typed details/differences, provider navigation, and cancellable background execution |
| Package data model | DONE | Exact identities, available-versus-unavailable fields, explicit bounded inventory/detail states, deterministic normalization, selection, stale correlation, search, dependency navigation, and typed event/effect mapping pass focused and baseline checks |
| Package data adapter | DONE | Validated shell-free discovery and exact batched `oe-pkgdata-util` commands return bounded typed inventory/detail data with unavailable fields, timeouts, cancellation, symlink/failure handling, and fake-process coverage. The real smoke was attempted but `build/tmp/pkgdata` is absent, so no live package-data compatibility is claimed |
| Package data workspace | DONE | Packages is a Navigator destination with correlated background inventory/detail execution, cancellation, search, stable selection, bounded dependency navigation history, exact recipe/provider actions, responsive explicit states, footer hints, and TestBackend coverage |
| Hardening matrix | DONE | Every integrated gate passes. With explicitly authorized temporary host sampling permission, matching perf 7.0.12 and cargo-flamegraph 0.6.13 captured the real deterministic headless workload into a nonempty 34 KiB SVG; kernel-symbol restrictions remain an honest host limitation |
| Operator documentation | DONE | The concise landing page retains guarded setup paths and embeds a real-binary, fixture-labelled UI demo plus the real perf-backed Flamegraph; the complete operator/troubleshooting and compatibility evidence remain linked and visual artifacts are validated |

## Priority queue

`RELVAL-FLOW-001` is active and depends on completed `RELVAL-KEYMAP-001`.

## Rules

- This document must agree with `docs/task-registry.toml`.
- Parent capability descriptions are not completion evidence.
- A task is `DONE` only after its verification command passes.
- Every intentional UI change updates `docs/ui-spec.md`.
- Every architecture change updates `docs/architecture.md`.
- Completed tasks should include the implementing commit in the registry notes.
