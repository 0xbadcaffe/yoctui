# Yoctui One-Stop Workbench UX Roadmap

Status: **approved implementation roadmap; implementation evidence is tracked in
`docs/task-registry.toml`**

Research snapshot: **2026-08-26**

## Product outcome

Yoctui should let an operator configure a Yocto environment, inspect metadata,
build and diagnose images, understand root filesystem composition, operate
interactive tools, and manage release evidence without losing context or
memorizing a different interaction grammar for every workspace.

The target is a beautiful, dense terminal workbench, not a collection of widget
demos. A visual element is admitted only when it improves comprehension,
navigation, safety, or speed with authoritative typed data.

## Non-negotiable constraints

- BitBake and the connected Yocto environment remain authoritative.
- Model reducers own selection, focus, scrolling, editing, animation, and job
  state. Renderers do not parse backend output or own durable interaction state.
- Unknown progress stays unknown. A gauge, meter, chart, or pie slice never
  fabricates a value.
- Every color or glyph has a text equivalent and works in no-color, ASCII, high
  contrast, reduced-motion, narrow, and screen-reader-oriented modes.
- Destructive operations retain an exact preview and explicit confirmation.
- Terminal applications receive input only through the existing writer lease and
  prefix-key boundary.
- Third-party code is added only after the license, MSRV, dependency graph,
  maintenance state, and Ratatui compatibility are rechecked at implementation
  time. Showcase screenshots and application source are not copied.

## Research baseline

The [Ratatui application showcase](https://ratatui.rs/showcase/apps/) demonstrates
several patterns that fit Yoctui:

| Pattern | Showcase references | Yoctui application |
|---|---|---|
| Contextual help and keyboard-first actions | [GitUI](https://github.com/gitui-org/gitui) | Generate Help, menus, palette, footer, and shortcut settings from one typed action catalog. |
| Focusable and zoomable telemetry panels | [bottom](https://github.com/ClementTsang/bottom), [bandwhich](https://github.com/imsnif/bandwhich), [trippy](https://github.com/fujiapple852/trippy) | Improve build/system telemetry without displacing task and diagnostic context. |
| Search, preview, then act | [television](https://github.com/alexpasmantier/television), [fzf-make](https://github.com/kyu08/fzf-make), [csvlens](https://github.com/YS-L/csvlens) | Use a consistent filter/result/Inspector/action flow for recipes, files, commands, artifacts, and history. |
| Pane-specific keymaps and editors | [rainfrog](https://github.com/achristmascarl/rainfrog) | Keep global keys small and stable while showing local keys for the focused pane or editor mode. |
| Tree exploration with guarded deletion | [dua](https://github.com/Byron/dua-cli), [yazi](https://yazi-rs.github.io/), [joshuto](https://github.com/kamiyaa/joshuto) | Use trees for layers, dependencies, and rootfs content; preserve multi-stage safety for cleanup. |
| List, detail, logs, and control | [oxker](https://github.com/mrjackwills/oxker), [taskwarrior-tui](https://github.com/kdheepak/taskwarrior-tui) | Keep selection, diagnostics, state toggles, and actions visible together. |
| Rich input with persistent status hints | [Codex](https://github.com/openai/codex), [oatmeal](https://github.com/dustinblackman/oatmeal), [steer](https://github.com/steer-ai/steer) | Make popup editors and terminal sessions obvious about mode, focus, ownership, and available actions. |

The [built-in widget showcase](https://ratatui.rs/showcase/widgets/) and
[third-party widget showcase](https://ratatui.rs/showcase/third-party-widgets/)
are inventories, not blanket adoption recommendations. Yoctui already provides
typed versions of several showcased behaviors.

## Interaction architecture

### One action catalog

Every operator action must have one stable ID and one catalog record containing:

- menu path and label
- concise description
- scope and required focus
- default shortcut plus optional aliases
- capability and selection requirements
- safety class and confirmation policy
- palette keywords
- footer priority
- Help grouping
- current availability and exact disabled reason

The application menu, context-action menu, command palette, Help, footer, mouse
hit targets, keybinding preferences, and automated keymap tests all project from
this catalog. They may present different subsets, but they cannot define actions
or labels independently.

### Menu model

`F10` opens a focus-trapped menu bar with these stable groups:

1. **Workspace** — profiles, environment verification, terminal sessions, quit.
2. **Build** — build options, tasks, logs, errors, cancellation, history.
3. **Navigate** — every Navigator destination plus back/forward/recent context.
4. **View** — pane focus, zoom, Inspector, filters, wrap/follow, theme.
5. **Tools** — Devtool, Raw Mode, SDK, QEMU, Wic, testing, security, QA,
   maintenance, and capability-aware utilities.
6. **Help** — contextual keys, command catalog, compatibility, diagnostics,
   operator guide, and version information.

Arrow keys move within a menu, `Enter` activates, `Esc` closes one level, and
typing performs bounded prefix selection. Disabled actions stay visible with
their exact reason. `a` or a right click opens the selected item's contextual
action menu using the same records. Menus never bypass existing confirmation.

### Default keybinding grammar

Existing F-key destinations remain stable. The new grammar removes local
surprises and adds missing navigation without stealing input from a terminal or
popup editor.

| Context | Default controls |
|---|---|
| Global | `F1` Help, `F2` Tasks, `F3` History, `F4` Dashboard, `F5` Logs, `F6` Layers, `F7` Recipes, `F8` Images, `F9`/`Ctrl+P` Commands, `F10` Menu, `q`/`Ctrl+C` Quit. |
| Pane focus | `Tab`/`Shift+Tab` cycle; mouse click focuses; Help shows the exact next and previous target. |
| Collection | `Up`/`k`, `Down`/`j`, `PageUp`/`PageDown`, `Home`/`End`, `gg`/`G`; mouse wheel follows the same bounded actions. |
| Tree | `Left`/`h` collapse or move parent, `Right`/`l` expand or move child, `Enter` opens, `Space` toggles when a checkable state exists. |
| Search | `/` edit, `Ctrl+U` clear, `n`/`N` next/previous, `Esc` clears before leaving the view. |
| Selection actions | `Enter` primary/open, `a` action menu, `Space` toggle, `?` contextual Help. Destructive letters are not the only route to an action. |
| Terminal | All keys go to the writer-owned PTY except the configurable `Ctrl+B` prefix; prefix Help always exposes detach, pane, session, copy/search, and literal-prefix routes. |
| Text editor | Explicit Normal/Insert/Visual modes own editor keys; the mode line always states how to save, preview, cancel, copy, paste, undo, and redo. |

Custom bindings are stored by action ID, not display label. Loading rejects
duplicate active chords in the same scope, reserved terminal-prefix conflicts,
unreachable critical actions, and invalid key sequences. A reset-to-default and
exportable keymap report are mandatory.

### Focus and scrolling

- Exactly one pane, subview, menu, dialog, palette, or terminal owns input.
- The focused title includes a textual marker in addition to border/color.
- Each workspace may define subfocus, but `Esc` moves outward predictably and
  `Tab` never enters a hidden or disabled target.
- A zoom action temporarily gives the focused work area the body while retaining
  the header, compact location breadcrumb, and footer; closing zoom restores
  exact pane, selection, and scroll positions.
- Every scrollable region uses the same row/page/top/bottom behavior and exposes
  retained position. Horizontal scroll is explicit and never changes selection.
- Follow mode, manual scrolling, search jumps, resize, filter changes, and
  retained-buffer eviction have reducer tests for exact offset behavior.

## Built-in widget plan

| Ratatui widget | Planned Yoctui use | Decision and fallback |
|---|---|---|
| `Block` and `Clear` | Pane shells, menus, dialogs, overlays | Keep shared Yoctui primitives; no new state. |
| `Paragraph` | Help, diagnostics, previews, editor projection | Standardize wrap, horizontal offset, selection, and scrollbar metadata. |
| `List` and `Table` | Navigator, menus, commands, tasks, packages, artifacts | Keep viewport virtualization and typed selection; add common row/page semantics. |
| `Tabs` | Workspace subviews such as rootfs composition modes and terminal sessions | Use only when all tabs remain keyboard/menu discoverable and narrow layouts expose the active tab text. |
| `Scrollbar` | Every overflowing bounded list, table, text view, and tree | Replace inconsistent title-only hints where a visible scrollbar materially helps. Keep textual position. |
| `Gauge` and `LineGauge` | Build, parse, task, job, disk, memory, sstate, test progress | Use determinate values only; always show percentage or numerator/denominator. |
| `Sparkline` | Compact CPU, RAM, I/O, network, task velocity | Preserve existing bounded typed histories and honest missing-sample gaps. |
| `Chart` | Zoomed telemetry/history and build velocity | Add only in expanded/zoom views; compact layouts retain values/sparklines. |
| `BarChart` | Largest rootfs directories/packages and artifact size comparison | Pair with sortable exact-byte table; collapse safely on narrow terminals. |
| `Canvas` | Optional dependency-graph overview or topology minimap | Use only if navigation, clipping, text equivalence, and performance beat a tree/table. |
| `Calendar` | Optional build/test history heatmap and support-renewal dates | Low priority; do not add merely for decoration. |

### Progress, gauges, meters, and throbbers

Progress is a hierarchy rather than one overloaded bar:

- build: completed/total tasks, average velocity, optional labeled ETA estimate
- parse/runqueue: authoritative phase and determinate fraction when supplied
- selected task: state, elapsed, worker/PID when authoritative, task percentage
- background job: lifecycle, operation-specific progress, cancel availability
- resource meter: CPU, RAM, build filesystem, and I/O/network rate with exact units
- cache meter: sstate reuse/hit/miss/unknown with provenance

Determinate progress uses gauges or meters. Indeterminate work uses one
`throbber-widgets-tui`-backed activity language if its adoption gate passes.
Reduced motion shows stable `Running`, `Loading`, or `Waiting` text, and all
themes use the same lifecycle words.

### Logs

The build Logs workspace keeps its typed bounded store instead of replacing it
with a generic logger widget. Improvements include:

- consistent row/page/top/bottom/horizontal scrolling
- virtualized render ranges for large retained buffers
- follow/pause state that is obvious in the title and footer
- search match count and next/previous position
- severity, build, recipe, task, source, and time-range filter chips
- bookmarks for retained diagnostics and jumps from Tasks/Errors/Jobs
- wrapped and unwrapped views with exact source and loss accounting
- export/copy through typed bounded effects

`tui-logger` is evaluated only for a separate Yoctui self-diagnostic view. It
must not capture or reinterpret BitBake domain logs.

### Text areas

The current reducer-owned popup editor grows into a reusable safe editor model:

- multiline Unicode text, cursor, selection, undo/redo, bounded history
- word/line/page motion, line numbers, wrap, search, replace, and mouse selection
- Normal/Insert/Visual modes with explicit mode line
- bracketed paste and clipboard effects
- validation diagnostics tied to exact ranges
- diff preview, conflict detection, atomic save, and recovery after save failure

`ratatui-textarea` was evaluated as an implementation reference and renderer
adapter candidate. The spike rejected adoption: its widget-owned mutable text,
cursor, selection, scrolling, history, and input state cannot round-trip the
complete reducer validation/search/diff/conflict/save lifecycle without a
second authority. Yoctui therefore retains the stateless custom renderer over
`TextAreaState`, with deterministic feature-parity tests and no candidate
dependency closure.

### Checkboxes and batch selection

Checkboxes are used for multi-select filters, build options, report selection,
package/rootfs drilldown, and batch-safe operations. Checked, unchecked,
indeterminate, disabled, and focused states each have text/ASCII equivalents.
`Space` toggles; `Enter` retains the primary action. Selection never implies
execution, and destructive batch actions still show every resolved target.

### Root filesystem structure and pie chart

The Images workspace gains a `Rootfs composition` subview with two explicit
authorities:

1. **Installed packages** — the exact image manifest correlated with bounded
   pkgdata sizes and provider identities.
2. **Filesystem tree** — an optional bounded, non-following scan of the exact
   BitBake-reported `IMAGE_ROOTFS` for the selected image/build identity.

The filesystem scanner rejects paths outside the build directory, does not
follow symlinks, accounts for hard links without double counting, identifies
special files, and reports entry/time/depth/byte bounds as limitations. Missing
or cleaned work directories are `Unavailable`, not an empty rootfs.

Wide views show a `tui-piechart` composition, legend, exact-byte/percentage
table, and drill-down tree. Medium views prefer bar chart plus table. Narrow,
ASCII, no-color, and screen-reader-oriented views use the sortable table/tree
only. Small categories collapse into an explicit `Other` slice whose members
remain inspectable. Package and directory views never mix their totals.

### Built-in terminal and `tui-term`

Yoctui already has a stronger-than-demo terminal architecture: daemon-owned
PTYs, a bounded `vt100` emulator, typed screen replicas, writer leases,
detach/reattach, split panes, copy/search state, and explicit termination.
`tui-term` currently renders a `vt100::Screen`; directly giving it raw terminal
bytes in `yoctui-ui` would duplicate parsing and violate the typed boundary.

The compatibility spike admitted `tui-term` 0.3.4 only through its generic
`Screen`/`Cell` renderer with every crate feature disabled. The daemon still
owns the only `vt100` parser. A validated sparse wire grid expands into one
bounded client replica, and the short-lived UI adapter projects it without raw
bytes or retained screen state. TestBackend and real-PTY evidence covers text,
Unicode width, styles, cursor visibility/position, scrollback coordinates,
resize, splits, and no-color behavior. The stripped reference link grew by only
160 bytes (0.1%); the shipped graph adds only `tui-term` over packages already
used by Ratatui. Full evidence is in
[`terminal-renderer-evaluation.md`](design/m21/terminal-renderer-evaluation.md).

The user-facing terminal work continues regardless of that dependency choice:

- a first-class Terminal Sessions destination and context-aware `Open terminal`
- obvious writer/read-only state and take-control action
- session tabs/list, split layout, zoom, rename, detach, close, and confirmed kill
- scrollback search, copy mode, selection, paste, and dropped-history accounting
- shell, devshell, menuconfig, SDK shell, Devtool editor, and Raw interactive
  session identities
- exact prefix help and literal-prefix forwarding
- reconnect and daemon-restart outcomes that never imply a process survived

### Dependency graphs, trees, scroll views, and variable-height lists

The completed dependency-graph spike rejects `tui-nodes`: the model-owned
bounded adjacency projection now drives stable selection, reverse anchors,
filtering, expansion, cycle/cross-edge reporting, numeric source positions, and
responsive topology/tree/table plus ASCII text without widget state. Its
48-package candidate closure therefore adds no parity value. The completed list/tree
spike rejected `tui-tree-widget`, `tui-scrollview`, and `tui-widget-list`:
Yoctui's stable-ID tree, `ScrollState`, and bounded variable-height viewport
already provide the complete external authority, cycle/depth/count limits, and
Unicode/ASCII text projection. Adopting their widget states would duplicate
selection, expansion, offset, or height authority.

### Optional image and large-text widgets

The completed [`ratatui-image` evaluation](design/m21/image-preview-evaluation.md)
rejects the candidate for the current deploy inventory: no artifact kind owns
raster MIME authority, probing mutates terminal input and may change tmux pane
state, threaded resize lacks Yoctui's bounds/cancellation, the closure adds 71
packages, and a stripped size-optimized reference binary grew 319,048 bytes.
Exact metadata and rootfs composition remain the deterministic fallback on
direct terminals, SSH, tmux, and TestBackend. `tui-big-text` is limited to
optional onboarding or an idle/empty canvas where it does not reduce density or
accessibility.

## Third-party dependency and license gate

The reusable admission gate now records an exact audited candidate snapshot in
[`compliance/widget-candidates.toml`](compliance/widget-candidates.toml) and its
resolved CycloneDX graph in
[`compliance/widget-candidates.cdx.json`](compliance/widget-candidates.cdx.json).
These are audit pins, not workspace dependencies. Every implementing task must
refresh its candidate before changing `Cargo.lock`.

| Crate | Audited version | SPDX license | MSRV reported | Decision |
|---|---:|---|---:|---|
| [`ratatui-image`](https://crates.io/crates/ratatui-image) | 11.0.6 | MIT | 1.86.0 | Rejected after transport/bounds/size spike; retain exact typed fallbacks. |
| [`ratatui-textarea`](https://crates.io/crates/ratatui-textarea) | 0.9.2 | MIT | 1.86.0 | Rejected after adapter spike; retain stateless custom renderer. |
| [`throbber-widgets-tui`](https://crates.io/crates/throbber-widgets-tui) | 0.11.1 | Zlib | 1.88.0 | Adopt without `rand`; model phase remains authoritative. |
| [`tui-big-text`](https://crates.io/crates/tui-big-text) | 0.8.9 | MIT OR Apache-2.0 | 1.88.0 | Defer until onboarding value is demonstrated. |
| [`tui-checkbox`](https://crates.io/crates/tui-checkbox) | 0.4.6 | MIT | 1.74.0 | Reject; native primitive is smaller than the dependency. |
| [`tui-logger`](https://crates.io/crates/tui-logger) | 0.18.3 | MIT | not declared | Reject; existing bounded tracing remains authoritative. |
| [`tui-menu`](https://crates.io/crates/tui-menu) | 0.3.1 | MIT OR Apache-2.0 | not declared | Reject; menus must project the typed action catalog directly. |
| [`tui-nodes`](https://crates.io/crates/tui-nodes) | 0.10.0 | MIT | not declared | Reject; bounded reducer-owned topology/tree/table projections have complete text parity. |
| [`tui-piechart`](https://crates.io/crates/tui-piechart) | 1.0.2 | MIT | 1.74.0 | Adopt for wide rootfs composition with exact text parity. |
| [`tui-scrollview`](https://crates.io/crates/tui-scrollview) | 0.6.7 | MIT OR Apache-2.0 | 1.88.0 | Rejected; retain reducer-owned scroll projection. |
| [`tui-term`](https://crates.io/crates/tui-term) | 0.3.4 | MIT | 1.86.0 | Adopt generic renderer only; all features disabled. |
| [`tui-tree-widget`](https://crates.io/crates/tui-tree-widget) | 0.24.1 | MIT | 1.86.0 | Rejected; retain stable-ID stateless tree renderer. |
| [`tui-widget-list`](https://crates.io/crates/tui-widget-list) | 0.15.3 | MIT | not declared | Rejected; retain bounded variable-height projection. |

MIT, Apache-2.0, and Zlib are already allowed by `deny.toml`, but allowlisting is
not sufficient compliance. Before adoption:

1. verify the crate and every enabled transitive dependency with `cargo deny`;
2. review default features and disable unnecessary native/image/backend features;
3. record exact copyright/license text in generated third-party notices;
4. keep `Cargo.lock`, source origin, checksum, MSRV, and Ratatui compatibility;
5. generate an auditable dependency/SBOM report for release artifacts;
6. reject missing, ambiguous, source-incompatible, or policy-disallowed licenses;
7. do not copy showcase assets, themes, screenshots, or application code;
8. if code is adapted rather than linked, retain all notices and provenance and
   require an explicit review in the implementing commit.

The workspace's generated shipped-dependency evidence is
[`compliance/THIRD_PARTY_NOTICES.md`](compliance/THIRD_PARTY_NOTICES.md) plus
[`compliance/yoctui.cdx.json`](compliance/yoctui.cdx.json). The verifier rejects
stale output, incomplete or dangling candidate graphs, invalid checksums,
implicit default features, and non-admitted candidates in the real manifests
or lockfile. The workspace graph also builds with `--locked --offline`.

## Delivery phases and progress

Progress counts required registry tasks, including the parent completion gate.

| Phase | Scope | Task IDs | Progress |
|---|---|---|---:|
| 0 | Research, visual acceptance, dependency/license policy | `UX-SPEC-001`, `UX-CONCEPT-VALIDATION-001`, `UX-LICENSE-001` | 3/3 |
| 1 | Action catalog, menus, keybindings, focus, scrolling | `UX-ACTION-CATALOG-001` through `UX-SCROLL-001` | 6/6 |
| 2 | Shared widgets, progress, telemetry, logs, editors, checkboxes, trees | `UX-WIDGET-PRIMITIVES-001` through `UX-LIST-TREE-001` | 10/10 |
| 3 | Dependency topology, rootfs composition, optional image preview | `UX-DEPENDENCY-GRAPH-001` through `UX-IMAGE-PREVIEW-001` | 5/5 |
| 4 | Terminal, dashboard, command center, onboarding, preferences | `UX-TERMINAL-EVAL-001` through `UX-PREFERENCES-001` | 5/6 |
| 5 | Responsive, accessibility, performance, PTY/live evidence, docs | `UX-RESPONSIVE-001` through `UX-DOC-001` | 0/7 |
| 6 | Parent completion gate | `UX-001` | 0/1 |
| **M21 total** | | | **29/38 (76.3%)** |

The historical product registry was 540/540 before M21. Registering these 38
tasks makes overall required progress **569/578 (98.4%)**. The research/spec,
six-scene production-renderer acceptance baseline, exact cell goldens, semantic
captures, executable implementation-gap ledger, and reusable dependency
admission/notices/SBOM/offline-build gate are complete. The validated
137-entry typed action catalog now drives global palette metadata/search,
contextual workspace actions, compatibility availability, and Help projection.
The versioned effective command keymap now validates scoped bounded chords,
preserves catalog defaults unless explicitly replaced, reserves the PTY prefix,
keeps critical routes reachable, routes through the app boundary, exports a
deterministic report, and migrates/persists atomically in the private session.
The Settings workspace now presents and edits that authority with scoped
search, textual states, trapped bounded capture, exact validation failures,
per-action/all reset, bounded export, atomic save, and retry.

## Test strategy

The pre-implementation visual-direction pack lives in
[`docs/design/m21/concepts`](design/m21/concepts/README.md). Its six PNG scenes
and manifest provide manual hierarchy, density, focus, palette, and affordance
anchors for Dashboard, Tasks, Errors, rootfs composition, the editor/menu, and
terminal sessions. The generated images are not exact goldens. Implementing
tasks must derive deterministic cell goldens from typed `TestBackend` fixtures
and may then rasterize those buffers with a pinned font for exact PNG diffs.
`./scripts/verify-m21-concept-pack.py` protects the concept pack's files,
dimensions, hashes, anchors, and lossless format.

| Layer | Required evidence |
|---|---|
| Model | Pure reducer tests for action catalog projection, key collisions/chords, focus/subfocus, zoom restore, every scroll transition, editor history, checkbox state, rootfs normalization, terminal presentation, and stale-generation rejection. |
| Protocol/backend | Version-compatible rootfs DTOs; fake Tinfoil/pkgdata/rootfs adapters; canonical-path, symlink, hard-link, special-file, size/count/time/depth bounds; cancellation and stale request tests. |
| App/input | Every keyboard, chord, menu, mouse, paste, and terminal-prefix route maps to the same typed action; modal/terminal focus traps and disabled actions cannot leak. |
| UI | Ratatui `TestBackend` at `200x60`, `160x50`, `130x40`, `100x30`, `80x24`, and below minimum; every theme, no-color, ASCII, high contrast, and reduced motion; semantic snapshots plus deliberately reviewed goldens. |
| Property/fuzz | Arbitrary selection/offset/content/resize sequences never panic, escape bounds, lose identity, or create inaccessible focus; rootfs input and terminal replica decoders stay bounded. |
| PTY | Real-terminal tests for F10 menus, palette, keybinding editor, focus/zoom, mouse scrolling, bracketed paste, terminal prefix/literal prefix, split sessions, copy/search, detach/reattach, and resize. |
| Performance | Extend the existing five-scenario matrix with menu-heavy, rootfs-large, graph-large, editor-large, and terminal-dense scenes; retain the existing 10 ms/frame ceiling and profile regressions before caching. |
| License/supply chain | `cargo deny check`, dependency feature audit, locked/offline build, third-party notice validation, SBOM generation, and source/checksum verification. |
| Live Yocto | Supported older/latest environments exercise menus and availability, a real build and cancellation, log correlation, image manifest/pkgdata/rootfs composition, context terminal, menuconfig/devshell where available, reconnect, and evidence expiry. |

## Milestone completion definition

M21 is complete only when:

- all 38 required M21 tasks are `DONE` and `./scripts/verify-roadmap.sh` passes;
- the action catalog is the sole authority for menus, palette, Help, footer,
  configurable bindings, and action availability;
- keyboard-only, mouse, no-color, ASCII, reduced-motion, narrow, and terminal
  session workflows are all usable and tested;
- progress, meters, charts, and rootfs visuals use authoritative typed values and
  retain textual fallbacks;
- the built-in terminal provides the complete session workflow without breaking
  the daemon writer/replica architecture, regardless of the `tui-term` spike
  outcome;
- every adopted dependency has a current compatible license/feature/MSRV review,
  locked source, notices, SBOM entry, and passing `cargo deny` gate;
- the UI performance matrix remains below the existing 10 ms/frame ceiling;
- live supported-Yocto evidence and operator documentation are current; and
- the unchanged full repository completion gate passes.
