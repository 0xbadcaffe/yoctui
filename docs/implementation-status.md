# Yoctui Implementation Status

The machine-readable source of truth is `docs/task-registry.toml`.

Status values:

- `NOT_STARTED`
- `IN_PROGRESS`
- `BLOCKED`
- `DONE`

## Current phase

M21 One-Stop Yocto Workbench Usability is active under the user's explicit
request. `UX-SPEC-001` is complete: the Ratatui application, built-in widget,
and third-party widget showcases were mapped to Yoctui's existing typed
workbench; the UI and architecture contracts now define one action catalog,
application/context menus, scoped keybindings, consistent focus/zoom/scroll,
progress and telemetry, bounded logs and editing, checkboxes, dependency and
rootfs visualization, the first-class terminal path, dependency/license policy,
38 atomic tasks, required tests, and measurable completion. M21 progress is
13/38 (34.2%); overall required registry progress is 553/578 (95.7%).
`UX-CONCEPT-VALIDATION-001` is complete: all six concept identities render from
typed fixtures through production `render_at` at `160x50`, with reviewed
semantic captures and exact cell/style goldens. The manifest checks semantic
anchors and maps 12 honest implementation gaps to incomplete owner tasks;
failure tests reject corrupt dimensions, missing captures or anchors, and gaps
whose owner is already complete. `UX-LICENSE-001` is complete: thirteen widget
candidates have exact versions, checksums, sources, features, MSRVs, Ratatui
compatibility, decisions, and a resolved CycloneDX graph; the real 236-package
workspace graph has generated notices/SBOM and passes `cargo deny` plus a locked
offline build. No candidate entered the workspace dependency graph.
`UX-ACTION-CATALOG-001` is complete: 25 global commands and 99 contextual
workspace actions now carry validated stable IDs, scope, menu/search/Help/footer
metadata, local and compatibility requirements, safety, and typed targets. The
palette and workspace presentations project the catalog, exact disabled reasons
compose local and daemon authority, and model/app/UI parity tests pass.
`UX-KEYMAP-MODEL-001` is complete: schema-versioned scoped preferences now
produce one deterministic effective command keymap with bounded typed chords,
workspace-before-global resolution, collision/prefix/reserved-key/reachability
validation, reset/report support, modal-safe app routing, and atomic migrated
session persistence. `UX-KEYMAP-UI-001` is complete: Settings now provides a
searchable scoped binding table, text default/custom/disabled/critical state,
focus-trapped bounded capture, exact conflict/reachability errors, remove and
reset controls, bounded report export, atomic save, and retryable failure. Typed
application and context menus are also complete: F10 exposes the fixed six-group
catalog projection, `a` and right click expose current-workspace actions, bounded
type-ahead and arrows stay focus trapped, exact local/capability denials and
safety classes remain visible, and activation reuses existing typed routes and
confirmation boundaries. Pane focus is now also complete: typed logical
subfocus and pane-only zoom retain exact selections and offsets, zoom has a
responsive textual breadcrumb, outward Esc restores one ownership layer at a
time, modal/terminal traps remain intact, and six read-only catalog commands
provide direct F10 View/palette focus, subfocus, and zoom access. Standardized
scrolling is also complete: one pure bounded row/page/edge/viewport contract,
one keyboard/mouse workspace dispatcher, real PageUp/PageDown/Home/End and
catalog `gg`/`G` routes, constant-time edge jumps, stable log identity across
filtering and eviction, retained Raw output bounds, textual responsive ranges,
and property tests now agree across model, app, production UI, and CLI. Shared
visual primitives are also complete: bounded typed projections drive exact-text
gauges/meters, histories, charts, bars, tabs, legends, and scrollbars through
semantic theme roles with explicit exceptional states and Unicode, ASCII,
no-color, high-contrast, reduced-motion, large-input, and narrow coverage.
Hierarchical progress is also complete: one typed projection keeps overall
build, parse, runqueue, selected task/job, resources, and unavailable sstate
authority distinct; exact fractions, honest unknown totals, partial ended
phases, frozen terminal values, and labeled estimates have model/app/production
renderer coverage. Accessible indeterminate activity is also complete: the
admitted Zlib throbber symbols use reducer-owned deterministic phase, no rand
feature, stable reduced-motion words, ASCII/no-color equivalents, and static
terminal outcomes. Telemetry polish is also complete: a pure six-series
projection preserves exact units and valid zeroes, distinguishes missing
current samples from wholly unavailable history, and drives both the compact
strip and expanded Tasks Context charts. Context zoom retains bounded Job
History and build-filesystem capacity, collapses to exact text at narrow
heights, and remains safe for empty, partial, maximum-sized, Unicode,
no-color, and reduced-motion inputs. Bounded log-explorer polish is next.

### Completed baseline through M20

M20 Raw BitBake Command Workbench supersedes the unrelated queue under the
user's explicit request. `RAW-REF-001` imported the supplied Wrynose 6.0 /
BitBake 2.18 cheatsheet verbatim at SHA-256
`ad95ecfa6a17691fa2a6d12f598f01fbd33de524c2a08ebccd218ef5fe88dd47` and
documents that it is a reference snapshot rather than runtime authority.
`RAW-SPEC-001` is complete: the authoritative UI and architecture contracts now
define hierarchy, exact selection-following help, typed/manual parameters,
bounded no-shell argv, exact previews, safety classes, daemon jobs versus PTYs,
detach/reattach, safe history/favorites, capability replacement, responsive
layout, mouse, and accessibility. `RAW-CATALOG-MODEL-001` is complete: Raw
Mode now has bounded stable identities, closed category/parameter/interaction/
safety types, shell-free typed argv parts, explicit execution policy, and
catalog validation for identity, text, placeholder, capability, and template
integrity. `RAW-CATALOG-001` is complete: the pinned built-in catalog contains
all 32 top-level/favorites
categories and all 464 commands from reference bash blocks. Its 288 direct
BitBake commands use typed shell-free argv templates; 176 pipelines,
redirections, setup snippets, and companion commands are explicitly inert.
`RAW-CATALOG-TRACE-001` is complete: its gate independently parses reference
bash blocks,
checks every source-line identity, heading, command, description,
classification, category, and rendered template, verifies the pinned SHA-256,
and rejects stale generated output. `RAW-CAP-001` is complete: Raw projections
consume only daemon-owned capability records and selected implementations,
distinguish all-of from any-of, preserve exact limitations and failure reasons,
and keep reference-only entries Unsupported. `RAW-CAP-PROBE-001` is complete:
31 version-agnostic Raw CLI capabilities use bounded direct
`bitbake --help` evidence, including a dedicated help-text probe for
multiconfig syntax. The daemon publishes positive, negative, or inconclusive
results once per environment generation and rejects stale results.
`RAW-PARAM-001` is complete: every declared kind parses into a closed typed
value with explicit byte/range bounds; required and optional emptiness remain
distinct; names, targets, paths, integers, and text reject option ambiguity,
control data, shell syntax, traversal, overflow, and kind disagreement.
Unicode path/text byte boundaries and every normal/failure class are covered.
`RAW-RECIPE-001` is complete: recipe and image identities come from current
typed inventories, targets combine current and newest-first history, tasks are
correlated to the exact selected recipe metadata, and configured
multiconfigurations come from the effective workspace value. Unavailable and
authoritative-empty inventories remain distinct, replacement is immediate,
and manual input continues through the same typed parameter validator.
`RAW-ARG-001` is complete: the shared popup editor now feeds a typed no-shell
tokenizer with explicit input, count, element, and aggregate byte ceilings.
It preserves quoted grouping and empty arguments, applies the documented
ordinary-character escape grammar, rejects every prohibited operator even
when assembled across quotes/escapes, and clears stale validated argv on edits
or replacement. `RAW-PREVIEW-001` is complete: the normalized catalog now
reconstructs each literal, required/optional parameter, joined/composed token,
explicit empty, and additional argument independently. Preview creation binds
the exact catalog version, daemon capability generation, authoritative build
directory, selected implementations, interaction/safety classes, and
limitation evidence; missing, stale, unavailable, or mismatched authority
fails closed. The TestBackend preview renders indexed native argv without a
joined command authority and degrades safely through 80x24 and tiny areas.
`RAW-MODEL-001` is complete: the pure stable-ID reducer owns category/command
selection, exact selection-following help, bounded search, typed form fields,
expert argv editing, preview/execution views, bounded history/favorite
selection, explicit favorite-removal confirmation, and view/focus restoration.
Catalog replacement clamps or removes stale identities; a safe capability
generation change closes only the stale preview and refreshes the form, while
capability/build-identity loss closes unsafe work with an exact notification
and no backend effect. `RAW-NAV-001` is complete: one Raw Mode identity now
crosses the typed screen and compatibility catalogs, the first `TOOLS`
Navigator row, command palette, shared focus and CLI input routing, Inspector,
help, contextual footer, and responsive shell. Navigation remains inspectable
when the daemon capability state is unknown, while the destination summary
uses the exact `bitbake.raw.cli` requirement. Responsive TestBackend coverage,
the reviewed target-design golden, and full model/app/UI suites cover the new
route. `RAW-CATEGORY-UI-001` is complete: the Raw reducer now projects a
browser order with Favorites pinned before the unchanged reference-derived
catalog order. The responsive Workspace renders a bounded, selection-following
category viewport with explicit position and textual `FAVORITES`, `BITBAKE`,
`REFERENCE`, `CONCEPT`, and `COMPANION` classifications; narrow terminals show
one active browser column and medium/wide terminals preserve both column
states. Keyboard aliases, no-color meaning, long labels, empty selection,
first/last bounds, and resize/search identity preservation are covered.
`RAW-COMMAND-UI-001` is complete: the command column consumes the reducer's
exact category, Favorites, or search projection and renders stable catalog
templates with textual `RUN`/`REF`, five-state capability, and favorite
markers. Selection and centered scrolling stay bounded across empty, stale,
large, Unicode, first/last, no-color, and responsive states; reference-only
rows remain inspectable without opening an execution form. `RAW-HELP-UI-001`
is complete: `Inspector: Raw command` follows the selected stable catalog ID
on every render and orders the exact reference description/section/template,
five-state availability, authoritative reasons and implementations,
interaction, safety, typed parameters, and favorite state. Executable,
disabled, and reference-only records remain explainable with explicit text;
empty/stale selection, long Unicode authority text, no-color mode, and wide,
medium-overlay, narrow, and below-minimum rendering are covered.
`RAW-FORM-UI-001` is complete: enabled executable selections open a shared
focus-trapped `Run BitBake Command` dialog bound to exact catalog, capability,
and build identities. Typed fields retain bounded Normal/Insert editors,
current selector/manual authority, inline validation, and the shell-free
Additional arguments editor; only a valid reducer request reaches the separate
indexed native-argv preview. Keyboard and pointer focus cannot escape the
modal, disabled/reference selections remain closed, and authority/build loss
closes unsafe work without execution. Wide, medium, `80x24`, no-color,
Unicode, empty/error, selector/manual, preview/back, and replacement paths are
covered. `RAW-EXEC-MODEL-001` is complete: five disjoint prefix-checked Raw
identity families, catalog-reconstructed confirmed intent, length-delimited
SHA-256 preview digests, and a pure correlated lifecycle now cover queued,
starting, running, cancelling, success, failure, cancellation, loss, elapsed
time, detach/reattach, exact job-or-PTY ownership, durable references, and
independently bounded stdout/stderr. Versioned bounded request, event, chunk,
result, and reconnect DTOs reject unknown required variants, malformed or
cross-kind identities, stale replacement, duplicate identity, Unicode byte
overflow, and inconsistent snapshots. The app performs only mechanical
wire/model conversion and atomically installs valid daemon replicas; no client
request contains executable, joined-command, PID, writer, or process authority.
`RAW-JOB-001` is complete: `Enter` on a reviewed noninteractive preview now
emits a typed daemon-only effect. The daemon reconstructs the built-in catalog
template and expert argv, rechecks the digest, generation, capabilities,
canonical BitBake executable, and build directory, then spawns native argv
without a shell. Its namespaced supervisor owns the process group independently
of client sockets, journals ordered queued/starting/running/cancelling/terminal
replicas, retains independently bounded Unicode stdout/stderr with explicit
drop/truncation counters, and maps success, nonzero, spawn failure, timeout,
graceful/forced cancellation, channel loss, and restart recovery exactly once.
Duplicate/stale requests and terminal or repeated cancellation fail closed;
reconnect installs current snapshots, while safe persistence strips output and
turns unrecoverable live ownership into detached `Lost`. `RAW-PTY-001` is
complete: confirmed interactive requests now take a separate typed daemon
route, are reconstructed against current catalog/capability/executable/build
authority, and enter the existing PTY actor only as immutable native argv.
Namespaced Raw PTY/session identities cannot collide with or be decoded from
generic sessions. Ordered Raw and PTY replicas cover start, writer-controlled
input/resize, detach/reattach, explicit termination, exit, loss, reconnect, and
cancellation without parsing terminal output or granting client process
authority. Cross-route, duplicate, stale, tampered, and identity-mismatched
starts fail before spawn. `RAW-OUTPUT-UI-001` is complete: the Raw execution
workspace renders exact command/request/owner identity, lifecycle, elapsed
time, result/exit/loss state, independently bounded stdout/stderr accounting,
and daemon PTY screens at every supported breakpoint. Follow/pause, bounded
Unicode-safe vertical/horizontal scrolling, local search counts, stream
selection, typed cancellation, explicit detach/reattach, and close-to-detach
routes are reducer-owned and tested. Attachment changes remain ordered daemon
events and never imply cancellation or terminal input. `RAW-HISTORY-001` is
complete: validated terminal replicas now produce versioned, request-unique,
newest-first history records bounded by count and aggregate encoded bytes. The
records retain only catalog/command identity, typed parameters, interaction,
timing, terminal outcome/exit, and safe durable references. Atomic daemon
persistence rejects malformed, oversized, unordered, duplicate, and unknown-
version history; restart recovery records abandoned work once as detached
`Lost` without owner, executable/build/capability authority, argv, messages,
output, or PTY state. Free-form text and file parameters are conservatively
omitted from the durable record. Current-catalog activation keeps stale entries
explicit and can only return to configuration before a fresh preview,
confirmation, and request. `RAW-FAVORITE-MODEL-001` is complete: one versioned
record per stable command retains a bounded name, canonical executable-template/
parameter/capability/interaction/safety digest, validated typed defaults,
validated additional argv, and explicit order under count and aggregate-byte
limits. Add, replace, rename, remove, and reorder publish only validated
collections. Current catalog projection preserves missing, changed-template,
and invalid-default records as stale and otherwise reports the current
five-state daemon availability without storing it. Activation reconstructs only
a fresh current-authority form with defaults; it retains no build/capability
authority, preview, request, job/session identity, output, or process state.
`RAW-FAVORITE-PERSIST-001` is complete: `session.toml` now defaults legacy
favorite data safely and validates the entire versioned collection before app
installation or persistence. The reader rejects symlinks and files above 1 MiB;
the writer validates/serializes first, uses a unique mode-`0600` temporary,
syncs file and directory around atomic rename, and preserves the prior file and
unrelated preferences on failure. Valid stale template digests remain explicit
rather than being discarded or upgraded. `RAW-FAVORITE-UI-001` now renders the
dedicated bounded Favorites workspace with typed inspect, reorder,
removal-confirmation, and fresh-form reopen actions. Rows retain stale records
and show template/default/argv details plus the current five-state
compatibility projection; mutations emit the existing atomic session
persistence effect. `RAW-SEARCH-001` is complete: bounded case-insensitive
search matches category, command identity, labels, and descriptions while
preserving exact selection and explicit no-match behavior. `RAW-MOUSE-001` is
active next.
`RAW-LIVE-001` is complete: the harness records clean local Poky BitBake
2.12.1/qemux86-64 recipe/dependency inspection, build completion, cancellation,
reconnect, native-argv read-only, and PTY read-only evidence. `RAW-DOC-001` is
complete and the Raw parent gate is complete; `UI-REGRESSION-001` is complete.
`RAW-COMPAT-001` is complete: the older and current live fixture matrices,
unknown-future evidence, replacement/stale projections, and zero-spawn denial
checks pass. The formerly blocked README and downstream UI work are complete.
`RAW-RESPONSIVE-001` is complete: Raw browser, Favorites, execution, and
below-minimum shells retain bounded selection and never panic across wide,
medium, narrow, and too-small terminals. `RAW-A11Y-001` is complete: no-color
and responsive Raw views retain semantic state, explicit focus, and modal/PTY
traps. `RAW-SECURITY-001` is complete: documented operators and hostile
argument values are rejected before spawn, Raw paths contain no shell launcher,
and the verification script passes native-argv coverage. `RAW-COMPAT-001` is
active next.
`RAW-MOUSE-001` now maps Raw category, command, Favorites, and history clicks
and wheel events through shared bounded row geometry while preserving modal and
PTY focus traps.
Runtime Raw Mode support will come only from the daemon-owned connected-
environment capability snapshot; shell pipelines, filesystem recipes,
companion commands, and conceptual sections in the reference are not
implicitly executable.

`LIVE-UI-POKY-001` is `DONE`. Supported Ubuntu 24.04/glibc 2.39 validation
built `core-image-minimal` for qemux86-64 from Poky
`d0b46a6624ec9c61c47270745dd0b2d5abbe6ac1` (`yocto-5.2.4`) with BitBake
2.12.1. The canonical manifest binds the tested binary and source commit to
startup, environment, recipes, layers, a real running task/log frame, build
completion, intentional typed failure, terminal, and daemon reconnect
captures. The earlier quota command was inapplicable because `/` has no user
quotas; the host-only pseudo symptom came from unsupported Ubuntu 26.04/glibc
2.43. `README-UI-001` is active next.

`README-UI-001` is `DONE`. The README now presents live running-task/log,
successful completion, and intentional typed-failure views generated
deterministically from the canonical semantic PTY captures. The SVG metadata
binds every view to the exact tested source, binary, Poky, BitBake, and target
identities, and `scripts/check-docs.sh` rejects missing or stale renders.
`UI-CLEANUP-001` is active next.

`UI-CLEANUP-001` is `DONE`. Stable rendering responsibilities now live behind
private `shell`, `widgets`, `workspaces`, `dialogs`, `telemetry`, `theme`, and
`layout` modules. The public renderer/theme API and strict target-design golden
are unchanged; all 214 UI tests and strict UI clippy pass.

`M13-UI-001` is `DONE`. All 48 required next-generation UI tasks and all 540
required product tasks are terminal. The aggregate gate passes the complete
workspace, semantic/golden/style, breakpoint, mouse, PTY, accessibility,
performance, strict Clippy, formatting, roadmap, product, and exact live Poky
evidence checks. Performance artifacts use an isolated release target so host
benchmarking cannot replace the supported-container binary bound into the live
manifest. The full non-weakened repository completion gate is the terminal
acceptance for this state.

`PTY-UI-TEST-001` is `DONE`: the audit found that daemon PTY output was
journaled but discarded by the interactive replica, leaving panes with session
metadata only. Real output now crosses the maintained emulator, a dimension/
cursor/row/scrollback/frame-bounded typed screen snapshot, the app replica, and
the matching client-local Ratatui pane without UI-side ANSI parsing. The real
PTY acceptance covers exact pane placement, resize, bounded scrollback,
attach/writer/detach lifecycle, `Ctrl+B d` focus retention, screen retention,
and uncorrupted workbench header/footer. Full workspace tests and strict clippy
pass.

`INPUT-TEST-003` is `DONE`: the app and runtime mouse suites use the same
geometry as rendering at wide, medium, narrow, and below-minimum sizes. They
cover pane focus, exact Navigator/task/tab/PTY selection, wheel-to-keyboard
routes, inert padding, dialog trapping, and supported split dragging with the
exact focused pane, real split axis, and bounded typed ratio delta.

`INPUT-TEST-002` is `DONE`: the e2e focus-flow sequence dispatches real
`Tab`/`Shift+Tab` actions forward and backward through Navigator, Workspace,
and Inspector while rendering 160x50, 100x30, and 80x24. It proves medium
replacement, narrow switcher visibility, selection retention, dialog/palette
trapping and exact restoration, plus terminal-session prefix isolation.

`INPUT-TEST-001` is `DONE`: the cross-crate `next_generation_keymap` e2e test
dispatches every shared function-key route into model state, global/focus/
Tasks/Logs bindings into exact typed actions, all ten `Ctrl+B` second keys,
and trapped modal confirmation routes. It rejects duplicate global bindings
and renders Help plus representative contextual footers to prove their labels
match the authoritative catalog. Specialized workspace routes retain their
existing focused typed-action tests.

`VISUAL-TEST-003` is `DONE`: an aggregate TestBackend suite enforces exactly
one focused-border corner across wide, medium, narrow, and modal states;
non-empty body section titles; every semantic status role; task lifecycle and
progress agreement; and disabled-action styling. It found and fixed a real
focus ambiguity where every Task Inspector subsection used the focused border.
Only the primary Inspector section now owns focus, with the subordinate facts,
output, actions, and system panes visibly inactive.

`VISUAL-TEST-002` is `DONE`: four reviewed 160x50 fixed-clock fixtures now
serialize every terminal symbol and style for idle Dashboard, active Tasks at
72%, selected failed `do_compile`, and daemon synchronization/degradation.
Ordinary tests report the first changed coordinate and cannot rewrite files;
the explicit update script regenerates only the four target fixtures and
prints their Git diff for review.

`VISUAL-TEST-001` is `DONE`: a deterministic 160x50 TestBackend semantic
catalog covers Dashboard, Tasks, Logs, Job History, Recipes, Layers, Images,
Settings, Build Environment, and a typed terminal-session pane. Stable region,
state, and control anchors cover standard, confirmation, destructive, result,
and editor dialogs; selected Tasks, Logs, Jobs, Recipes, Layers, and Settings
rows must also retain semantic selection styling. The contract deliberately
ignores unrelated blank-cell changes while keeping anchor changes reviewable.

`RESPONSIVE-UI-001` is `DONE`: a canonical semantic TestBackend matrix now
covers 200x60, 160x50, 130x40, 100x30, 80x24, and 79x23 across Dashboard,
Tasks, Logs, Recipes, Layers, all three pane-focus states, and the typed Build
Options dialog. It proves wide pane priority, medium Inspector replacement,
narrow switching, selected-state retention, complete controls, clean text,
and an exclusive below-minimum resize screen without mutating focus or
workspace selections.

`PERF-UI-002` is `DONE`: recipe and layer renderers now compute a pure centered
viewport from authoritative selection, filtered row count, and visible height,
then construct Ratatui rows only for that bounded range. Query/inventory shrink
recomputes immediately, so no retained cache or stale invalidation surface is
needed. The same matrix reduced large metadata from 5.318 to 0.789 ms/frame
(85.2%); idle, active-build, log-heavy, and telemetry cases remain below 0.84
ms/frame. Model and TestBackend tests prove deep selection, query replacement,
inventory shrink, empty state, and layer selection cannot retain stale rows.

`PERF-UI-001` is `DONE`: a deterministic 160x48 matrix measures idle at 0.396
ms/frame, active build at 0.595, large metadata at 5.318, log-heavy at 0.845,
and maximum bounded telemetry history at 0.643 against an explicit 10 ms
ceiling. The fresh measured-worst-case flamegraph contains 12,843 samples over
6,000 frames, checksum `f4d850b421930dfd`, zero unresolved/null SVG frames,
and 0.3596% policy-filtered malformed raw event weight. Symbolized stacks put
`recipes` at 32.69B weighted inclusive events versus 5.66B for `layers`; the
renderer constructs rows for all 4,096 recipes before Ratatui clips the table.
That real hot path is assigned immediately to `PERF-UI-002`. The harness,
validator unit test, and fresh flamegraph gate pass.

`A11Y-UI-001` is `DONE`: the buffer-level accessibility contract and aggregate
invariant suite now exercise task, log, history, health, dialog, high-contrast,
no-color, and reduced-motion meaning at wide, medium, and narrow sizes. The
suite found and fixed two real issues: Tasks workspace focus had no bold
attribute-only border, and the minimum-size Build options dialog clipped its
Esc hint. Determinate progress remains numeric, indeterminate reduced-motion
state remains textual, all no-color cells reset color while selection uses
reverse video, and animation frames cannot change reduced-motion output. The
reviewed golden now correctly dims the unfocused Tasks border. Focused tests,
all 193 UI tests, the full workspace, strict clippy, and 43 bridge tests pass.

`MOUSE-UI-001` is `DONE`: one geometry-owned router now matches the documented
header/shell/footer and wide/medium/narrow pane regions. Navigator and task
rows, Security/QA/Testing tabs, dialog picker wheels, workspace wheels, and PTY
leaves all emit typed actions; PTY selection carries the exact pane identity,
and split dragging derives the bounded delta from the focused split's actual
axis and ratio instead of coordinate parity. Modal input is trapped and
unsupported/header/footer/minimum-screen coordinates are inert. Three focused
app tests, the CLI mouse-runtime test, all 192 UI tests, the full all-feature
workspace suite, strict clippy, 43 bridge tests, docs, and roadmap checks pass.

`DIALOG-UI-001` is `DONE`: reusable render-only dialog shell, tone, style,
bounded-geometry, field, validation, and control primitives now unify every
typed dialog outer shell. Modal, confirmation, destructive, result, and error
identity is title-first so it survives narrow clipping; selection and
validation use textual markers; forms reserve validation/control space; and
editors retain their typed clipboard/navigation controls. Focus trapping,
nested transitions, exact-pane restoration, asynchronous FIFO dialogs, and
all typed confirmation behavior remain model-owned and unchanged. Focused
UI/model tests, all 192 UI tests, the full all-feature workspace suite, strict
clippy, and 43 bridge tests pass.

`PALETTE-UI-001` is `DONE`: one focus-trapped overlay now uses documented
centered/inset geometry, shared counted search, wide
`Command | Shortcut | Availability` columns, fact-preserving narrow rows,
marker-plus-text compatibility/local availability, exact bounded detail, and
a selected/total visible-row window. Empty/long queries, local disablement,
wide/medium/minimum layouts, high contrast, monochrome, no-color, and reduced
motion pass. All 188 UI tests, the full workspace suite, strict clippy, and 43
bridge tests pass without changing the canonical shell golden.

`SEARCH-UI-001` is `DONE`: one responsive search line now projects explicit
`EDITING`/`FILTERED`/`IDLE` state, a cell-bounded query and text cursor,
selected/total result counts, and truthful navigation across metadata, logs,
packages, image/SDK artifacts, test results, Security, QA, Compatibility, and
the command palette. `Ctrl+U` is decoded once and maps to a distinct typed
clear action in every domain; active-search footer/help hints agree. Long and
wide-character queries, no matches, wide/medium/narrow layouts, command focus,
high-contrast, no-color, and reduced-motion pass. Focused model/app/CLI/UI,
all 183 UI tests, the full workspace suite, strict clippy, 43 bridge tests,
docs/roadmap checks, and the unchanged canonical golden pass.

`FOOTER-UI-002` is `DONE`: one model-owned projection now prioritizes exact
errors, pending confirmations, notifications/results, daemon/BitBake
synchronization, and local background/build activity. Arbitrary notification
strings remain informational unless exact retained log or typed build outcome
state supplies severity. The footer normalizes and ellipsizes marker-plus-text
status in a responsive slot before the clock, preserves Help/Menu/Quit,
removes the old workspace-obscuring notification popup, names stale state
without claiming reconnect, and keeps no-color/reduced-motion meaning. Focused
model/UI tests, all 300 model tests, all 181 UI tests, full workspace, strict
clippy, 43 bridge tests, docs/roadmap checks, and the reviewed canonical golden
pass.

`FOOTER-UI-001` is `DONE`: one model-owned F1-F10 catalog now drives app
dispatch, Help, and truthful footer labels. The old false `F3 Jobs`,
`F4 Terminal`, and `F9 Search` labels are gone. The persistent rail measures
whole current-context hints, reserves Help/Menu/Quit, omits redundant
current-screen routes, switches to explicit modal controls, hides the clock
below 100 columns, and preserves compact SDK/Testing/Security/QA controls.
Focused shared-catalog, keymap, modal, width, theme/no-color/reduced-motion,
all 179 UI tests, and the reviewed 160x48 footer-row golden pass.

`HEADER-UI-001` is `DONE`: the persistent header now separates authoritative
project identity from build target, adds marker-plus-text build state, and
reserves independently measured daemon/BitBake health. Full, wide, compact
wide, medium, and narrow tiers progressively hide DISTRO/release, MACHINE,
project, and BitBake without clipping higher-priority identity. Missing paths
and target are explicit, absent optional metadata is omitted, and non-current
replicas suppress retained BitBake lifecycle. Focused breakpoints, stale,
high-contrast, no-color, reduced-motion, all 178 UI tests, and the reviewed
160x48 golden pass.

`SYSTEM-UI-002` is `DONE`: reusable semantic health projection now gives every
daemon, BitBake, compatibility, filesystem, log-pressure, and workspace state
a text marker plus semantic role. Stale/non-current authority suppresses
retained BitBake state in both System Status and the persistent header. Disk
health shares explicit 70% warning/90% error thresholds with the gauge; log
eviction and evicted errors escalate warning/error, and unknown workspace is
named. High-contrast, no-color, reduced-motion, responsive, all lifecycle,
threshold, degraded compatibility, pressure, and shell cases pass; all 176 UI
tests and the reviewed 160x48 golden pass.

`SYSTEM-UI-001` is `DONE`: System Status is now a bounded four-line projection
of daemon connection/version availability/uptime, BitBake lifecycle/version,
active jobs, PTYs, clients, compatibility generation/counts, build-filesystem
capacity, and workspace identity. Daemon-owned values render only for a
Current replica; stale/synchronizing/disconnected replicas expose unavailable
facts instead of retained counts. The missing daemon version is explicit,
queue-depth and imprecise memory remain omitted, compact lines ellipsize, and
the reviewed 160x48 golden records the denser pane. Focused current/compact/
stale/theme tests and all 175 UI tests pass.

`METRICS-UI-006` is `DONE`: one bounded telemetry strip now composes the
semantic gauges and rate histories in Dashboard and tall Tasks workspaces.
Wide uses stable CPU/RAM/Build-FS/Read/Write/RX/TX order, medium uses
CPU/RAM/Build-FS plus a two-line disk I/O cell, and below 64 columns the strip
is hidden in favor of the existing Inspector/System Status route. Optional
disk/network groups are omitted without reserving misleading cells; the
eight-row tier appears only at 46+ workspace rows. Exact breakpoints, optional
sources, high-contrast, no-color, reduced-motion, Dashboard, and Tasks
integration pass focused coverage, and all 174 UI tests pass.

`METRICS-UI-005` is `DONE`: optional network RX and TX now render separately
with binary bytes-per-second text and independently scaled bounded-history
sparklines using their semantic graph roles. An unavailable current delta is
named while prior valid history may remain visible; a real zero delta remains
distinct. Compact rendering prioritizes text, unsupported hosts remain honest,
and no reset/first/interface-change sample is manufactured. Wide, compact,
current, unavailable, zero, high-contrast, no-color, reduced-motion, and
cockpit fixtures pass focused coverage.

`METRICS-UI-004` is `DONE`: disk read and write now render separately with
binary bytes-per-second text and independently scaled bounded-history
sparklines using their semantic graph roles. An unavailable current delta is
named while prior valid history may remain visible; a real zero delta remains
distinct. Narrow labels prioritize text, and no reset/first/device-change
sample is manufactured. Wide/narrow/current/unavailable/zero/high-contrast,
no-color, reduced-motion, and cockpit fixtures pass focused coverage.

`METRICS-UI-003` is `DONE`: build-filesystem capacity now uses one responsive
semantic gauge only when both the configured build directory and a consistent
capacity sample exist. Wide labels retain free/total plus the exact build path;
medium labels retain free/total, and narrow labels retain the numeric used
percentage. Missing context and invalid samples say unavailable rather than
fabricating zero. Wide/medium/narrow/minimum, invalid, high-contrast, no-color,
reduced-motion, dashboard-summary, and cockpit fixtures pass focused coverage.

`METRICS-UI-002` is `DONE`: RAM now uses one responsive semantic gauge with an
overflow-free whole used percentage, full and shared-unit used/total labels,
and a compact narrow label. Missing, zero-total, and inconsistent samples say
unavailable instead of fabricating zero. The same overflow-free calculation
feeds bounded history. Wide, medium, narrow, huge-capacity, invalid,
high-contrast, no-color, reduced-motion, cockpit, and model-history fixtures
pass focused coverage.

`METRICS-UI-001` is `DONE`: the dashboard uses one dedicated semantic CPU
gauge with numeric percentage, full and compact authoritative core-count
labels, and a minimal narrow label. Missing utilization now says unavailable
instead of implying ongoing sampling or rendering zero, and a missing core
count is omitted instead of guessed. Wide, medium, narrow, high-contrast,
no-color, reduced-motion, and existing cockpit fixtures pass focused coverage.

`METRICS-MODEL-002` is `DONE`: one reducer-owned telemetry history now bounds
CPU, RAM, disk read/write, and network RX/TX independently to the latest 60
valid samples. The CLI derives disk rates from the exact build-filesystem
device and network rates from the active lowest-metric IPv4 default-route
interface using measured intervals. First observations, reset/nonmonotonic
counters, identity changes, missing sources, zero intervals, and overflow stay
unavailable and append neither spikes nor synthetic zeroes. Focused model and
CLI parser/rate tests pass.

`METRICS-MODEL-001` is `DONE`: a closed typed provenance catalog records the
source, units, host support, nominal cadence, delta requirement, bounded
history, precision, renderability, and unavailable behavior of 17 host/daemon
metrics. The audit confirms CPU/RAM/build-FS/load and typed daemon facts,
withholds the first CPU delta and initially identified disk/network rates as
uncollected. It also
identifies daemon queue depth as a client-count alias and resident memory as
page-size-assumption diagnostic data; neither is renderable. System Status no
longer labels the alias as queue depth. Focused model/CLI tests, all 169 UI
tests, and the documentation gate pass.

`INSPECTOR-UI-003` is `DONE`: one reusable action list now renders aligned
typed names and shortcuts, explicit marker-plus-state availability, semantic
enabled/disabled styles, and exact reasons, limitations, and implementations.
Tasks orders the real `c/l/h/B` actions, exposes the inactive-cancel reason,
and keeps useful actions visible in reduced-height Inspector layouts. `B` now
dispatches through the app keymap rather than a duplicate CLI branch. Focused
UI/app tests, all 169 UI tests, and the reviewed canonical golden pass.

`INSPECTOR-UI-002` is `DONE`: a borrowed model projection now joins the selected
task to an exact recipe version, typed dependencies and paths, and a bounded
exact-identity log tail without cloning retained records. The Task Inspector
shows authoritative lifecycle, progress, timing, worker/PID, log, and
dependency facts; PR/workdir and every other missing source stay explicitly
unavailable. Aggregate waiting and responsive modes remain honest. Focused
model/UI tests, all 167 UI tests, and the reviewed canonical golden pass.

`INSPECTOR-UI-001` is `DONE`: model-owned Inspector modes now distinguish the
selected entity across every workspace and Navigator focus. The UI uses exact
typed titles plus one optional semantic section order for primary/secondary
facts, related paths, recent output, contextual actions, and system status in
wide, overlay, and narrow layouts. Task subpanes use the same names. Focused
model/UI and all 166 UI tests pass; the reviewed golden changed only the two
Task section titles.

`JOB-UI-002` is `DONE`: a pure bounded summary separates queued from executing
background work, counts exact failures and all retained terminal completions,
excludes build-history duplicates, and reports daemon-owned records only for a
Current replica. Embedded and standalone Job History titles share the same
wide/compact textual projection. Focused model/TestBackend coverage and the
reviewed canonical golden pass.

`JOB-UI-001` is `DONE`: one borrowed model projection combines bounded
background jobs and retained build records, pins nonterminal work first, and
bounds reducer selection across that shared ordering. Embedded and standalone
tables use exact marker-plus-text lifecycle states, responsive column tiers,
authoritative context/timing only, active-row emphasis, explicit unavailable
and empty states, and selection-driven bounded detail. Focused model/UI tests,
all 164 UI tests, and the reviewed canonical golden pass.

`LOG-UI-002` is `DONE`: one width-aware textual projection now drives both the
Logs activity row and embedded Tasks Log Viewer. Follow and pause are always
named; Filtered, Search, and Evicted appear only when active. Wide labels add
the exact query, warning/error eviction loss, and coalescing, while compact
labels retain every required state word without relying on color or animation.
Focused TestBackend coverage and all 163 UI tests pass; the reviewed canonical
golden now records the embedded `▶ Follow` state.

`LOG-UI-001` is `DONE`: Logs now uses dedicated activity and Viewer sections.
The Viewer title carries exact selected recipe/task context, follow state, and
clamped vertical/horizontal positions. Trace/info/warning/error rows carry text
markers and semantic styles; normalized case-insensitive search hits receive
accent emphasis, including attribute-only no-color treatment. Retained-empty
and filter-empty states differ. Existing typed full-entry copy is visible for
any selection, while source-log open appears only for an authoritative path.
ANSI normalization remains in the backend adapter. Focused projections and
TestBackend tests plus all 293 model and 162 UI tests pass.

`TASKS-UI-003` is `DONE`: identified `TaskQueued` events now remain `Queued`
without a false start time or PID, while total-derived unidentified work stays
an aggregate `Waiting` row. The waiting filter intentionally includes both.
Seven unique marker-plus-text labels distinguish queued, waiting, running,
succeeded, failed, cancelled, and lost states; semantic attributes survive
no-color mode, selected rows keep the one dominant selection treatment, and
reduced motion never changes lifecycle meaning. Focused model and TestBackend
tests cover the state transition, aggregate distinction, filter, every label,
and no-color rendering.

`TASKS-UI-002` is `DONE`: a pure model `BuildSummary` now drives two compact
Tasks header rows. Known nonzero totals render a strong numeric gauge;
unknown totals render explicit indeterminate activity and never `0%`. Typed
active/waiting/warning/error/elapsed values follow, with aggregate pace/ETA
only where honestly derivable and space permits. Terminal elapsed freezes from
the retained build record. Sstate reuse is omitted because no authoritative
source exists. Focused model/UI tests pass, and the reviewed 160x48 golden was
intentionally refreshed for the real summary.

`TASKS-UI-001` is `DONE`: the table selects its complete column set before row
construction. Narrow renders Task/Status/Progress, medium adds Recipe/Time,
and wide adds Worker/PID only when typed rows supply them. Active work receives
a full-row running treatment without masking selection; determinate bars stay
numeric, while unknown progress remains explicit and freezes to stable active
text under reduced motion. No per-task CPU or ETA column exists. Focused
model/UI tests and the unchanged 160x48 cell/style golden pass.

`NAV-UI-001` is `DONE`: the complete workspace rail now uses model-owned stable
groups, collapse/expand state, bounded visual-row projection, full-row semantic
selection, a current/total scroll title, compatibility markers, and only
authoritative Tasks/Errors/Logs/Devtool badges. QEMU / Wic is explicitly
reachable. Keyboard and size-aware mouse hit-testing emit the same typed
selection, group-toggle, and activation actions; the canonical Tasks context
tree selects real owning workspaces and never simulates filesystem behavior.
Focused model/app/UI and CLI mouse tests plus the unchanged literal golden pass.

`FOUNDATION-UI-003` is `DONE`: the public `SemanticTheme` catalog now resolves
surface/text, borders, selection, lifecycle, emphasis, bounded graphs,
informational paths, headings/tables, and source-preview meanings for all nine
named/compatibility themes. Production renderers use the named roles without
hardcoded colors; no-color resets every color while retaining focus,
selection, and severity attributes. Focused UI/model tests, the unchanged
160x48 cell/style golden, formatting, and warnings-denied UI Clippy pass.

`FOUNDATION-UI-002` is `DONE`: `yoctui-ui::primitives` now provides render-only
pane shells, section headers, focused/inactive selection, separators,
marker-bearing statuses, shared state views, bounded scroll labels, and stable
priority-based responsive columns. Five focused tests, the unchanged 160x48
cell/style golden, formatting, and warnings-denied UI Clippy pass. The module
accepts resolved text/styles only and owns no model or backend state.

`FOUNDATION-UI-001` is `DONE`: `docs/ui-spec.md` now fixes the exact region
hierarchy, 80x24 minimum, 200/160/130/100/80 breakpoints, deterministic width
and height degradation, pane priority, Inspector collapse, footer/status and
telemetry behavior, responsive table columns, focus/scroll indicators, shared
empty/loading/unavailable/error grammar, and explicit omissions for every
unsupported illustrative field. No renderer changed before the contract.

The requested next-generation TUI milestone is registered as M19 because the
repository already has a completed, evidence-backed M13. All requested task
IDs are retained, including the `M13-UI-001` parent gate. The completion gate
requires an independent M19 registry and acceptance matrix before the
pre-existing product, compatibility, quality, profiling, and live-evidence
gates can pass.

`M19-GOV-001` is `DONE`: the task graph preserves all 458 existing task records
and statuses, documents the milestone-number reconciliation, selects the first
dependency-eligible specification task, and wires the new independent gate
without weakening any older check.

`FINAL-GATE-PERF-001` remains `DONE`. The seventh clean candidate passed the complete
repository gate, including CLI authority smoke, workspace tests and Clippy,
fuzz/stress, ASan/LSan, Rust and Python coverage, audit/deny, Valgrind, release
profiling, fresh perf sampling, and live-evidence validation. The final capture
contains 2,014 samples, 6,000 deterministic frames, the stable
`95f340a128cd6012` checksum, no lost samples, and zero unresolved/null output
frames. Non-interactive sudo could not restore the host policy; an operator must
run `sudo sysctl -w kernel.perf_event_paranoid=4` to restore the original value.

`PERF-FLAMEGRAPH-QUALITY-001` is `DONE`: the replacement 160x48 benchmark
drives 6,000 deterministic production reducer/Ratatui frames without a daemon
or Yocto checkout. Its final portable DWARF capture records 2,014 real samples,
the stable `95f340a128cd6012` checksum, and no lost samples. Two malformed raw
call-chain lines (0.2394% of event weight) are reported and
excluded below the 0.5% ceiling; the SVG has zero unresolved/null frames. A
bounded task-table viewport and one shared borrowed task projection removed
full-inventory formatting, cloning, and repeated sorting, reducing the same
workload from roughly 6.3 seconds to 4.028 seconds in the final capture. The
exclusive costs
are expected Ratatui buffer/Unicode work, with shared task sorting at 2.66%.
The validator, format, Clippy, full workspace tests, all 43 bridge tests,
documentation, and roadmap checks pass.

`UI-WIDE-RAIL-001` is `DONE`: F1–F10 now remains visible on every screen at
130 columns or wider; compact layouts retain contextual actions and the exact
canonical 160×48 golden is unchanged. All-label tests cover Dashboard and
Tasks at 130, 160, 180, and 200 columns. All 140 UI tests, the full workspace,
Clippy, 39 bridge tests, PTY snapshots, docs, roadmap, and the live 1,829-recipe
Poky gate pass. The installed release matches the local artifact at SHA-256
`c55c2d73…4fbe4924`.

`UI-LITERAL-001` is `DONE`: the canonical 160x48 shell, strict 7,680-cell style
golden, typed mixed Navigator, task/log/history cockpit, structured Inspector,
F1–F10 routes, transactional theme preview, and live 1,829-recipe Poky workflow
all pass. Formatting, the complete Rust workspace, Clippy with warnings denied,
all 39 Python bridge tests, documentation, and roadmap validation pass. The
independent `FINAL-GATE-PERF-001` host-policy blocker remains explicit.

`UI-LITERAL-LIVE-001` is `DONE`: the release client passes the controlling-PTY
gate against `~/src/poky/build` with 1,829 recipes. F2 enters the canonical
Tasks cockpit with live project categories, F10 reaches Choose theme,
WhiteClassic persists, bridge diagnostics do not corrupt the alternate screen,
and terminal restoration is clean. Formatting, the complete workspace, Clippy,
all 39 bridge tests, docs, and roadmap checks pass.

`UI-LITERAL-UX-001` is `DONE`: F1–F10 decode distinctly and invoke their named
typed routes; B remains the build-options key. Tab focus remains shared, F10
opens the menu containing Choose theme, Enter persists a preview, and Esc
restores the original theme/color state. All 139 UI tests and wide PTY capture
pass.

`UI-LITERAL-COCKPIT-001` is `DONE`: center tiers are exactly 17/18/9 rows and
Inspector sections 16/15/7/6 rows. Typed tasks render in observed chronology,
the selected log and metadata remain authoritative, and retained typed jobs
populate history. All 241 model and 138 UI tests pass.

`UI-LITERAL-NAV-001` is `DONE`: canonical Tasks renders live layers, recipes,
images, task families, and MACHINE as the reference tree, with a typed
layer/job/PID footer and full-row selection. Compact layouts retain the complete
destination catalog. Model, app, and all 137 UI tests pass.

`UI-LITERAL-SHELL-001` is `DONE`: the canonical Tasks scene has exact
26/89/45 body columns, shared-edge header/footer framing, the stable F1–F10
rail, and the approved near-black/blue/amber/lime/cyan/red DarkPro hierarchy.
All 136 UI tests and the strict visual golden pass.

`UI-LITERAL-HARNESS-001` is `DONE`: the typed reference scene renders through
an injected clock into a compact symbol/style golden. Normal tests compare all
cells and identify the first changed coordinate; only the explicit update
script rewrites the artifact for review.

`UI-LITERAL-SPEC-001` is `DONE`: the authoritative UI and architecture
documents now define exact canonical region geometry, the mixed typed project
tree, stable F-key rail, deterministic cell/style comparison, and the one
intentional semantic correction from BitBake Idle to Running for an active
task.

`UI-FOCUS-ROUTING-001` is `DONE`: pane focus now consumes only mapped focus
actions, and non-dialog notifications consume only `Enter`/`Esc`. `Ctrl+P`,
help, build, and screen-specific keys continue through normal routing. Focused
CLI/app tests, formatting, Clippy, docs, and roadmap checks pass; a real
controlling-PTY probe confirms that `Ctrl+P` opens and traps input in the
command palette.

`UI-STARTUP-DIAG-001` is `DONE`: bridge stderr is bounded and non-obscuring,
pane/global key routing no longer drops input, and the exact installed release
passes live theme selection with 1,829 Poky recipes. Formatting, the complete
workspace, workspace Clippy, all 39 Python bridge tests, docs, and roadmap pass.
Only the independent host perf-policy blocker remains.

`UI-STARTUP-LIVE-001` is `DONE`: the shell-resolved 0.1.0 executable
was an older registry install despite sharing the source build's version. It
has been replaced with the verified release binary and the operator session's
bridge/color preferences restored. The controlling-PTY live gate rejects raw
BitBake startup diagnostics and drives `Ctrl+P` → `Choose theme` →
`WhiteClassic`, verifying persistence with 1,829 recipes. The installed and
local release artifacts are byte-identical at SHA-256 `8fdf5201…d1e6982`.

`UI-STARTUP-STDERR-001` is `DONE`: the Rust bridge now pipes and continuously
drains stderr into a sanitized 16 KiB tail rather than inheriting the terminal.
Routine BitBake notes, warnings, and shutdown traces cannot corrupt Ratatui;
failed handshakes and query disconnects retain bounded context. Focused tests,
all 185 `yoctui-bitbake` tests, formatting, and focused Clippy pass.

`UI-LIVE-COLOR-AUTHORITY-001` is `DONE`: Yoctui's resolved color mode now
controls Crossterm, so ambient `NO_COLOR` cannot contradict Color=true while
explicit `--no-color` still selects the attribute-only widget palette. The
live 1,829-recipe colored PTY gate passes with `NO_COLOR=1` inherited; focused
theme/startup tests, isolated snapshots, Clippy, formatting, and roadmap pass.

`UI-LIVE-RECOVERY-001` is `DONE`: normal startup is session-safe and
metadata-capable, expected daemon absence no longer obscures the workbench,
theme selection and pane focus are explicit, and the real colored Poky gate
passes with 1,829 recipes. Formatting, the full workspace, Clippy, all 39
bridge tests, docs, roadmap, and isolated PTY snapshots pass. The only
remaining registry task is the independent host perf-policy blocker.

`UI-LIVE-POKY-001` is `DONE`: the private-XDG live gate passed against
`~/src/poky/build` with Poky 5.0.19, qemux86-64, DISTRO poky,
core/yocto/yoctobsp, and 1,829 recipes including core-image-minimal and busybox.
The colored 160x48 PTY contains grouped workbench and explicit focus anchors
without the old daemon fallback notice. Workspace tests, Clippy, and all 39
bridge tests pass.

`UI-LIVE-DISCOVERY-001` is `DONE`: `Choose theme` in the command palette opens
the shared named picker, selecting a theme enables color unless `--no-color`
locks it, and Settings explains that lock. Wide and medium command rails name
the active, next, and previous pane; narrow layouts retain their explicit pane
switcher so contextual actions remain visible. Model, app, all 134 UI tests,
and PTY snapshots pass.

`UI-LIVE-STARTUP-001` is `DONE`: legacy session backend state no longer
overrides the bridge default, `--no-color` no longer overwrites stored color,
snapshot subprocesses use private XDG roots, and expected daemon absence does
not obscure the local workbench. Focused tests and PTY snapshots pass, with a
before/after digest proving the operator session remained unchanged. The
previously corrupted local preference was restored to dark-pro with color and
the bridge enabled.

`UI-LIVE-RECOVERY-SPEC-001` is `DONE`: startup overrides are launch-scoped,
test XDG roots are private, expected daemon absence stays in persistent status,
the local bridge remains metadata-capable, and theme/focus controls have
explicit discoverability and live-Poky acceptance contracts.

`UI-VISION-001` is `DONE`: the approved one-line shell, grouped Navigator,
task/log/history cockpit, structured Inspector, and contextual command rail
ship together and render only typed state. Formatting, all workspace tests,
workspace Clippy, all 39 bridge tests, documentation, roadmap validation, and
real PTY snapshots pass. The independent host perf-policy blocker remains
explicit and does not invalidate the redesign acceptance.

`UI-VISION-RESP-001` is `DONE`: compact headers preserve project and
daemon/BitBake anchors at 80 columns; breakpoint, reduced-height, all-theme,
and no-color TestBackend coverage passes. Narrow, medium, and wide real-PTY
semantic snapshots are refreshed. All 132 UI tests, the full workspace,
workspace Clippy, 39 bridge tests, docs, and roadmap checks pass.

`UI-VISION-TASKS-001` is `DONE`: wide Tasks now shows the dense task table,
bounded selected-task typed log tail, and retained background/build history.
Its Inspector is divided into task metadata, recent log, actual shortcuts, and
daemon/BitBake/system status. Reduced height drops history first while keeping
the primary table and log. Focused tests, all 130 UI tests, model job tests,
and Clippy pass.

`UI-VISION-NAV-001` is `DONE`: Navigator destinations now appear under
`OVERVIEW`, `CONTENT`, `BUILD`, `VALIDATE`, and `TOOLS` headings with semantic
amber hierarchy, full-row selection, and bounded scrolling that keeps the
last destination visible at 80x24. The typed screen order matches the visual
groups; focused model/UI tests, all 128 UI tests, and Clippy pass.

`UI-VISION-SHELL-001` is `DONE`: the oversized telemetry banner is replaced by
a one-content-line bordered project/MACHINE/DISTRO and daemon/BitBake header.
The footer is now a semantic contextual key rail with a fixed-width clock;
medium-width hints remain readable. All 126 UI tests and UI Clippy pass.

`UI-VISION-SPEC-001` is `DONE`: `docs/ui-spec.md`, architecture, roadmap, and
the task registry now capture the approved reference's dark panel grid,
blue selection, lime progress, amber hierarchy, one-line operational header,
context command rail, and task/log/history composition.

`FINAL-GATE-PERF-001` was initially blocked on host perf policy after the other
release checks passed. Temporary sampling access subsequently produced strict
resolved evidence and the complete gate passed; the terminal record is at the
top of this document. The published 0.1.0 release was not affected.

`CRATESIO-COVERAGE-001` is `DONE`: Python coverage now measures the canonical
bridge bundled in `yoctui-bitbake`, while Ruff and mypy inspect both the
packaged source and external tests. Ruff, formatting, mypy, all 39 bridge
tests, and packaged-source coverage pass at 75.95%.

`CRATESIO-PUBLISH-001` is `DONE`: `yoctui-model`, `yoctui-protocol`,
`yoctui-bitbake`, `yoctui-ui`, `yoctui-app`, and `yoctui` 0.1.0 are published
on crates.io. A clean locked install from registry sources reports
`yoctui 0.1.0`, exposes complete help, and completes a headless inspection
through the embedded bridge. Published package VCS source commit
`6c66b4777d05a7f45e105d0cb955eb3e5a322a7d` is the `v0.1.0` release tag.

`DAEMON-UPGRADE-LIFECYCLE-001` is `DONE`: lifecycle validation now accepts
Linux's exact ` (deleted)` process-image suffix only when its remaining path
equals the private runtime record. Unrelated paths and repeated suffixes remain
foreign. Focused lifecycle tests, the workspace, Clippy, docs, and roadmap
checks pass.

`DAEMON-ATTACH-QUIT-001` is `DONE`: Workspace now shares the typed global
`q`/`Ctrl+C` route already used by Navigator and Inspector. The deterministic
terminal lifecycle probe is isolated from unrelated user daemons, and a real
attached-PTY check exercised `q` plus the required active-build `Y`
confirmation, restored the alternate screen, and left daemon job 1 running.
Focused, workspace, Clippy, all 39 bridge, docs, and roadmap checks pass.

`DAEMON-ATTACH-BUILD-001` is `DONE`: daemon-owned builds now use the typed
bridge, compact workspace/build/parse/task/terminal events into bounded attach
state, restore that state through the normal reducer, and continue applying
ordered incremental progress. Attached workspace authority now selects the
Dashboard and connected environment without starting a competing local backend
or falling back to `/`. Graceful daemon stop refuses active jobs instead of
silently orphaning them. Focused snapshot/replica/mapping tests, the full
workspace, Clippy, all 39 bridge tests, docs, and roadmap checks pass. The
optimized installed binary is running daemon instance
`a68edcb8ebf4694191776e2a2fde3256`; live release attach reported bridge, Poky
5.0.19, `BB Running`, `core-image-minimal`, and authoritative `94/4090` task
progress before the client detached and left the build running.

`TELEMETRY-COCKPIT-001` is `DONE`: Dashboard and Tasks now provide
terminal-native CPU, memory, disk, load, history, task-velocity, ETA, and
high-resolution progress meters. The persistent daemon parent gate is `DONE`;
the implementation and focused
verification for `BRIDGE-PROGRESS-001` pass. Its terminal `DONE` state now has
complete-gate evidence including a fresh real perf-backed Flamegraph.
Fractional Scarthgap `ProcessProgress` values now normalize to bounded wire
integers, PID-only `TaskProgress` records reuse build-scoped task identities,
and determinate task progress renders as a bar in both Dashboard and Tasks.
Real Poky acceptance, collision-safe daemon fixtures, responsive PTY delivery,
Configuration UI coverage, and global quit routing pass. The synchronized
ten-second real-terminal probe passes ten consecutive runs. On 2026-08-15 the
operator temporarily enabled userspace perf sampling and the required fresh
Flamegraph capture passed. The resumed full gate reached the `yoctui-bitbake`
suite, where `cli_control_cancels_the_owned_process_group` did not report its
expected graceful cancellation under the parallel run. Its fixture now waits
for an explicit post-trap readiness marker before cancellation; the focused
test passes 100 consecutive runs and all 180 library tests pass. No independent
registry task remains incomplete.
The full completion command otherwise exited 0, but pytest-cov printed that its
exact 74.58% result misses the required 75% threshold while incorrectly
returning status 0. Bridge failure-path coverage must be raised until the report
itself passes; the faulty status is not accepted as completion evidence.
Typed invalid-request coverage now checks empty build targets and malformed
recipe, variable, dependency, source, metadata, and filter identities. It found
and fixed the vacuous empty-target acceptance. All 38 bridge tests pass and the
coverage report itself clears the threshold at 75.37%.
The complete rerun then reached `yoctui-bitbake`, where the QA-layer
graceful/forced cancellation test hit `ETXTBSY` while spawning a rewritten
fixture. No independent registry task is eligible while fixture publication is
made race-free. QA-layer execution now retries only `ETXTBSY` with four bounded
attempts and a 5 ms delay. The cancellation test passes 100 consecutive runs,
the classifier is covered, and all 181 BitBake library tests pass; the complete
gate passes.

Release-quality, utility-workbench, embedded-shell, and CI workflow tasks are
complete. In-app build-environment onboarding is now in progress: it will let
operators select or clone a source, initialize it safely, use an interactive
setup shell when needed, and verify BitBake before build controls unlock.
Build environment is now a dedicated Navigator destination and unconfigured
startup focuses Navigator there; typed profile editing and verified image
inventory are complete, with semantic theme repair complete as well.
The typed profile and correlated connection gate are complete; adapter-backed
initialization now validates paths and captures a bounded child-only
environment, with interactive-required detection. Reviewed clone setup is the
active next step.
Reviewed Poky clone requests now have exact vectors, destination safeguards,
and fake-git coverage; the setup UI is the active next step.
Settings now visibly reports Build environment state and verification guidance,
with a typed verification shortcut and responsive TestBackend coverage. The
no-argument CLI startup now creates an unconfigured session instead of using
the current directory; Settings verification now executes the typed
initialization adapter and dispatches correlated outcomes. Managed backend
installation now uses the captured environment after typed workspace
verification. A follow-up UX sequence is active to move Build environment out
of general Settings, restore Navigator startup focus, unlock typed images only
after verification, and correct theme rendering.
Yoctui now uses Packrat's eight built-in palettes—Dark Pro, White
Classic, Matrix Green, VSCode Dark/Light, Accessible Dark, Soft Light, and
High Contrast—while retaining `--no-color` as a separate accessibility override.
Legacy `dark` and `light` configuration values remain supported as aliases.
The README now uses one guarded Poky build-environment path and explicitly
sets `BUILDDIR` before sourcing `oe-init-build-env`.

## Current task

See `docs/current-task.md`.

## Milestone summary

| Milestone | Status | Notes |
|---|---|---|
| M0 Governance | DONE | Contracts, registry, active-task handoff, and repository reconciliation are in place |
| M1 BitBake cockpit | DONE | The telemetry cockpit, progress compatibility, and full completion gate pass with fresh real perf evidence |
| M2 Persistent workbench | DONE | Persistent shell, responsive modes, focus, dialogs, palette, preferences, notifications, background jobs, and all specified workspaces pass their parent gates |
| M3 Development workbench | DONE | Layers, Recipes, Configuration, Devtool, dependency why-built, signatures, and the typed package-data workspace are complete |
| M4 Images/SDK/QEMU/Wic | DONE | Images, SDK, QEMU, Wic creation, and protected device writing pass their cross-layer parent gates |
| M5 Testing/QA/Security | DONE | Unified Testing, Security, and QA cross-layer parent gates pass; fake evidence remains separate from live compatibility |
| M6 Maintenance | DONE | Typed Sstate, Services, Release, and optional integrations pass their atomic, cross-layer, and milestone parent gates |
| M7 Hardening | DONE | Fuzz, stress/process-tree, ASan/LSan, property, terminal, Valgrind, deterministic profile, real perf-backed Flamegraph, honest Python coverage, transient-spawn handling, deterministic cancellation, CI, documentation, and completion integration pass |
| M12 crates.io distribution | IN_PROGRESS | Bundle the bridge, prepare the public package graph, then publish and clean-install `yoctui` 0.1.0 |

## Reconciliation evidence

| Capability | Status | Evidence and remaining work |
|---|---|---|
| Persistent application shell | DONE | Header, Navigator, Workspace, Inspector, and Footer remain visible during builds (`8769017`, `b3e7452`); breakpoint TestBackend coverage is in `733a593` |
| Responsive layouts | DONE | Wide three-pane mode, medium Inspector overlay, narrow visible pane switcher, too-small messaging, resize preservation, and all-screen boundary tests are complete |
| Focus routing | DONE | Bidirectional pane cycling, modal input trapping, nested-modal return targets, exact pane restoration, quit cancellation, and responsive focus rendering are covered |
| Dialogs | DONE | One typed FIFO queue drives build, image, recipe, Devtool, BBMASK, editor, quit, and completion workflows; invalid actions are inert and asynchronous completion waits behind active input |
| Command palette | DONE | Typed catalog, case-insensitive search, contextual availability, disabled explanations, inert invalid activation, focus restore, themes, and narrow rendering are covered |
| Themes | DONE | Five complete semantic palettes cover shell, focus, selection, status, severity, progress, dialogs, notifications, and syntax; monochrome/no-color use terminal attributes |
| Task animation | DONE | UI-tick fast/slow cadence, stable reduced-motion activity, honest unknown progress, and nonanimated determinate bars/terminal rows have reducer and TestBackend coverage |
| System telemetry | DONE | Typed CPU, memory, filesystem capacity, core-count, and load samples drive semantic gauges and 60-sample CPU/RAM sparklines; average task velocity, ETA, compact A/W/F counters, and fractional-cell task bars are responsive and tested |
| Background-job model | DONE | Stable IDs, typed lifecycle/context/progress/result/error, bounded output/history, cancellation capability, and reducer coverage are implemented |
| Background build execution | DONE | Confirmed builds allocate one job; typed events drive lifecycle/output; navigation persists; failure, cancellation rejection/acknowledgement, and backend loss are covered |
| Live BitBake bridge | DONE | Finite progress is bounded at the bridge boundary, PID-only task progress uses active-build worker identities, stale progress is ignored, and installed read-only inspection passes against BitBake 2.8.1 / Poky 5.0.19 |
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
| Tasks workspace | IN_PROGRESS | Existing typed task state and filters remain; determinate per-task bars are being added once live PID-only progress reaches the model |
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
| Hardening matrix | DONE | Every integrated gate passes, including fresh perf, deterministic signal-ready CLI cancellation, honest Python coverage, and bounded QA-layer transient spawn handling |
| Operator documentation | DONE | The concise landing page retains guarded setup paths and embeds a real-binary, fixture-labelled UI demo plus the real perf-backed Flamegraph; the complete operator/troubleshooting and compatibility evidence remain linked and visual artifacts are validated |

## Priority queue

Configuration, BBMASK, build target, Wic creation, SDK publication, SDK native
tools, Testing launch, result import, and result comparison now use the shared
bounded popup editor. The shared migration gate is complete; JUnit export is
verified on that path, and the remaining QA, security, and maintenance
operations popup migration is split by subsystem and workflow family; Security
and QA report imports are complete, and Maintenance sstate form migration is
split into readiness and cleanup tasks; both now use the shared bounded TOML
editor without weakening typed preview, candidate discovery, or destructive
confirmation, and their parent gate passes; service import/export now uses the
shared popup while retaining capability-owned context. Maintenance
release forms are likewise split into locked cache, build history, and Git
archive tasks; all three now use the shared popup while retaining their typed
preview and local/network separation, and the release parent gate is next.
The editor agenda now has explicit adoption, typed state, routing/rendering,
and migration tasks; JUnit export and the full operations parent gate pass.
Optional project-profile specification is now active.

M10 optional project profiles and M11 persistent daemon/session architecture
are specified as required dependency-ordered implementation queues. Neither is
implemented or live-validated yet; the optional profile behavior and trust
boundary are specified, and the pure typed version-1 model now validates
favorites, presets, workflows, portable paths, bounds, duplicates, references,
and explicit stale/ambiguous resolution without a command-string escape hatch.
Safe optional loading and explicit generation now pass CLI and app tests with
bounded TOML, schema validation, symlink rejection, atomic no-clobber writes,
confirmed replacement, inert startup state, and typed generation effects.
Profile rendering and interaction now expose team-intent rows in Build
environment with explicit resolution states, keyboard selection, authoritative
favorite navigation, and preset-to-existing-confirmation routing. No profile
selection starts work. A fresh isolated Poky Scarthgap clone now passes both
optional no-profile and explicit-profile paths through the real bridge and
BitBake 2.8.1 metadata inventories; all five representative team-intent items
resolve authoritatively. The recorded metadata-only host limitations do not
claim an image build. README now documents the optional schema, safe example,
portable team intent, user-local settings, inert loading, fail-closed trust
rules, BitBake authority, and headless inspection. M10 is complete; the M11
daemon/client architecture specification is complete.
The architecture gate now fixes the daemon as owner of BitBake, long-lived
jobs, PTYs, global sequencing and safe persistence; clients own only terminal
presentation and typed intent. It specifies secure local Unix IPC, gap-free
attach/reconnect, explicit stop/restart, honest crash and host-reboot states,
multi-client and single-writer arbitration, SSH-local attachment, and a shared
implementation path for explicit standalone mode. Typed daemon protocol work
is complete with a separate bounded length-prefixed wire format, negotiated
versions/capabilities/limits, typed identities, attach/replay snapshots,
ordered events, correlated generation guards, job and PTY state, writer
epochs, layout/mouse messages, confirmations, heartbeat, resync, and explicit
errors. Secure local Unix IPC is complete.
Local IPC now passes real Unix-socket tests for canonical private paths,
same-UID peer authentication, mode enforcement, symlink/non-socket rejection,
owned stale cleanup, reconnect, disconnect, frame limits, and deadlines. It
contains no TCP listener and fails closed where native peer credentials are
not implemented. Daemon lifecycle commands are complete.
Lifecycle commands now pass process-level start/status/restart/typed-stop and
SIGTERM cleanup tests using the one Rust binary, authenticated runtime records,
boot/executable/instance liveness checks, safe stale recovery, and no shell
daemonization. Client auto-attach remains intentionally gated on client parity;
systemd user-service integration is complete.
Optional systemd user-service integration is complete with safe atomic unit
generation, one-binary foreground execution, shell-free no-root user-manager
operations, explicit enablement, unsafe-file rejection, and a tested direct
fallback diagnostic. Migration of authoritative long-lived state into the
daemon was split before implementation into typed partition, job-family
migration, runtime ownership, and parent-gate tasks. The typed partition is
complete with checked revision/generation, validated bounds, authoritative
workspace/environment/profile/BitBake/recovery state, replaceable replicas,
and independent client presentation. Long-lived job-family migration is
complete: the daemon boundary now reuses the existing typed build,
background-job, task/history/log, artifact, SDK, testing, security, QA,
maintenance, QEMU, and Wic models plus bounded PTY metadata, while replica
installation deliberately leaves screen, focus, theme, editors, and
notifications client-local. Foreground runtime ownership is complete: the
authenticated daemon instance initializes state through the checked reducer,
advertises snapshot capability, and returns the same authoritative typed
snapshot after a real client detach/reattach while remaining alive. Gap-free
incremental synchronization remains separately gated. The daemon-state parent
gate passes.
Snapshot synchronization is complete with atomic checked watermarks, bounded
snapshot/event/log retention, exact same-instance replay, explicit replacement
for expired or invalid cursors, and a client synchronizer that rejects gaps and
withholds stale resume cursors.
Safe persistence is complete with a versioned private atomic user-state file,
strict schema/size/type/ownership checks, reconstructable workflow/session
metadata, optional logs, and an enforced prohibition on persisted live-process
claims.
Restart recovery is complete: validated state seeds a new daemon instance,
formerly nonterminal jobs and PTYs become `Lost`, writer/viewer/client liveness
is cleared, history and names remain visible, profiles require reload, and
BitBake remains disconnected pending a supported probe.
Host-reboot behavior is complete with an explicit changed-boot boundary,
honest `Lost` session visibility, typed metadata-only relaunch intent, and
documented unprivileged user-service enable/login/lingering guarantees. The
host-reboot gate is complete.
The daemon-owned BitBake controller abstraction is complete with typed
contexts, observations, capabilities, sessions, transitions, generations,
timeouts, restart/reconnect composition, and explicit failures independent of
the UI. Supported BitBake socket integration is complete: the Unix adapter
validates endpoint ownership and type, delegates the native process-server
protocol to workspace Tinfoil, correlates bounded commands, and reports typed
capabilities, identities, timeouts, and server loss. Live compatibility remains
reserved for the real-Poky acceptance gate. Shell-free BitBake CLI control is
complete with capability-authorized exact server-control argv, captured
environment execution, bounded output, deadlines, process-group cancellation,
and typed outcomes. Controlled BitBake restart is complete with exact active-job
confirmation, stale-state rejection, bounded controller orchestration, and a
typed authoritative metadata refresh. Daemon-owned PTY architecture
specification is complete, including ownership, byte transport, environment,
resize, attach/detach, single-writer, termination, scrollback, copy/search,
paste, recovery, and security semantics. The typed PTY session model is
complete with checked identity/context/lifecycle, dimensions, bounded
scrollback metadata, live ownership, exit state, viewers, and epoch-protected
single-writer control. The Unix runner is complete with real PTY ownership,
raw bounded I/O,
resize, child sessions/process groups, graceful and forced termination, honest
exit status, and real-PTY coverage. Maintained terminal emulation is complete
through a bounded typed `vt100` wrapper covering terminal
cells/styles, cursor, alternate screen, scrollback, resize, bracketed paste,
application and mouse modes without ad-hoc parsing. PTY attach/detach is
complete with real-process detach survival, current-screen reattach,
writer release on client loss, and honest Running/Exited/Lost listings. The
bounded multi-session registry is complete with monotonic IDs, unique names,
independent client selection, close/termination coordination, history and
resource limits.
Typed Yocto PTY contexts are complete for build/source/layer/recipe/Devtool/
SDK/deploy identities with canonical path and captured-environment validation;
project profiles cannot supply commands. Interactive Devtool PTY routing is
complete for authoritative workspace shells and exact `edit-recipe` while
noninteractive actions remain managed jobs. Menuconfig/devshell PTY routing is
complete with exact authoritative recipe/task validation, current kernel and
U-Boot provider resolution, verified build environments, previewed argv, and
fail-closed stale handling. Safe persistent SDK/native environment shells are
complete with digest-revalidated setup-file capture in an isolated bounded
child environment, exact installed-SDK and native-build routes, and no parent
environment mutation. The attachable Ratatui client gate was split into atomic
transport, replica, and runtime-ownership migrations; typed client transport is
active next. The transport now negotiates the bounded protocol over same-user
Unix IPC, attaches/resumes with verified instance watermarks, exposes typed
events and correlated command results, answers heartbeat messages, and waits
for explicit detach acknowledgement. Replica installation is active next.
Replica installation is complete: typed daemon BitBake/job/PTY/client/log/
recovery state renders in the persistent header while every presentation field
remains client-local across replacement and disconnect. Interactive runtime
wiring now attaches, polls snapshots/events without blocking terminal input,
routes standard build/cancel effects to correlated daemon commands, and
detaches explicitly. The runtime gate was further split because existing
Devtool/SDK/QEMU/Wic/testing/QA/security/maintenance/utility coordinators still
own processes in the client; migrating those job families is active next.
That migration is split by existing typed boundaries: Devtool is active first,
followed by SDK/QEMU/Wic and then testing/QA/security/maintenance, with a final
client-shutdown ownership gate. No generic command payload is permitted.
Devtool migration is complete with a closed wire enum, canonical context
validation, the existing shell-free command spec, daemon-owned real runner and
process group, sequenced job/log state, correlated cancellation, and detach-
survival coverage. SDK/QEMU/Wic ownership is active next.
The artifact migration is further split along its distinct safety boundaries:
SDK is complete: closed typed operations carry exact artifact/tool/environment
identities, daemon reconstruction reuses the validated adapter, and the daemon
owns output, timeout, cancellation and terminal state. QEMU is active next.

`UX-POPUP-EDITOR-002` is complete with the model-owned editor boundary and
reference rendering. `UX-POPUP-EDITOR-003` added typed selection and bounded
undo; input/rendering integration is now active.
The integration work is split into input normalization and shared rendering;
input normalization is active.
Input normalization and shared rendering are complete. The JUnit reference
popup now exercises reducer-owned cursor/selection, Unicode-safe movement,
Home/End, selection replacement, clipboard copy, internal and bracketed paste,
and a shortcut row that remains visible at every supported breakpoint. Existing
TOML workflow migration is split into atomic build, configuration, target, Wic,
SDK, and Testing tasks. Build environment and clone now use shared model state,
key routing, cursor/selection rendering, and clipboard behavior while retaining
their typed apply/review gates. Configuration and BBMASK now use the same state,
key, cursor/selection, and clipboard paths while retaining allowlisted previews
and explicit writes. Build-target editing now shares cursor, selection,
navigation, and clipboard behavior while its requested task remains read-only
and its build confirmation remains mandatory. Wic creation now shares the
editor path, selects its output directory, and reserves responsive validation
space while retaining all typed Wic preview gates. SDK publication now shares
the editor path and immediate destination replacement while retaining exact
path validation and confirmation. SDK native tools now share multi-field
editing while retaining FindSysroot versus RunNative restrictions and exact
previews. Testing launch now shares navigation and clipboard behavior, renders
validation in-popup, and enforces its authoritative context. Testing result
import and comparison share those editor behaviors while preserving absolute
import paths and current-inventory comparison identity resolution.

## Rules

- This document must agree with `docs/task-registry.toml`.
- Parent capability descriptions are not completion evidence.
- A task is `DONE` only after its verification command passes.
- Every intentional UI change updates `docs/ui-spec.md`.
- Every architecture change updates `docs/architecture.md`.
- Completed tasks should include the implementing commit in the registry notes.
embedded shell session model.
embedded shell session model is complete; the active task adds the native PTY
backend.
sessions with Yocto context and utilities.
sessions with Yocto context and utilities are complete; the active task adds
embedded shell PTY end-to-end coverage.
### Daemon-owned QEMU jobs

Confirmed runqemu launches now use a closed typed daemon request. The daemon
reconstructs and validates the image identity, executable, launch modes,
memory and bounded arguments before starting the existing process-group-aware
runner; output, cancellation and terminal state are published independently
of the attached client.
### Daemon-owned Wic creation

Wic image creation now crosses the daemon boundary as a closed typed request;
the daemon reconstructs the kickstart/output identity and reuses the existing
validated Wic command and bounded process runner.
### Daemon-owned Wic device writes

Confirmed device writes now submit exact image/device identities to the daemon,
which revalidates the device through the existing inspector before owning the
process group, output, cancellation and terminal state.
### Daemon-owned selftest sessions

Selftest requests now cross the daemon boundary as closed typed identities.
The daemon reconstructs the validated TestRunnerAdapter command, owns the
process group and publishes bounded output, cancellation, timeout and loss
events independently of the attached client.
### Daemon-owned test-result import

Result imports now execute in the daemon and publish bounded typed snapshots.
Comparison remains explicitly client-local until its request and diff payload
are represented in the versioned protocol.
The daemon test cache now retains both bounded wire snapshots and authoritative
import records by generation, enabling safe comparison worker reconstruction.
Daemon test comparisons now resolve retained authoritative result generations,
compute bounded typed transitions, and publish versioned comparison diff events.
Layer QA checks now use a daemon-owned typed runner: confirmed executable/layer
identities are revalidated at the daemon boundary, bounded output is published
as daemon logs, and cancellation plus terminal/lost states survive client
detach.
QA capability inspection is now routed from the interactive client through the
bounded daemon request, preserving selected recipe identity and workspace
roots while keeping capability/task execution out of the client process.
QA report imports now use a daemon-owned bounded worker with typed generation,
path, cancellation, report snapshot, and terminal job events.
Security/CVE/SPDX report scans now use a daemon-owned bounded worker with typed
generation and path requests, cancellation, report snapshots, and terminal
security job events. Confirmed cve-check-map-pkgs operations now cross a typed
daemon boundary with revalidated executable/input identities, bounded output,
cancellation, and terminal/lost security job state.
Maintenance capability inspection now crosses the daemon boundary with typed
build/sstate metadata and bounded tool/limitation snapshots; sstate execution
is the next maintenance split gate.
Confirmed sstate readiness now reconstructs the validated capability/preview on
the daemon and owns runner output, cancellation, and terminal state; service
and release operations remain the next split gate.
Release/build-history/signature/archive tool discovery is now included in the
daemon maintenance capability snapshot; the release runner remains next.
Release/utility operations now map to daemon-owned validated external runners
with bounded output, cancellation, and terminal/lost state.
The migrated job families now share one typed daemon routing path; runtime
ownership/detach integration is the next gate.
The complete attachable client gate is now verified across transport, replica,
runtime routing, and UI daemon-health rendering; standalone policy is next.
Standalone fallback is explicitly local-only: attach failures produce a visible
diagnostic and preserve the existing single-process UI without implying daemon
job ownership.
Multi-client protocol state is typed and bounded, including client identity and
client-local replica boundaries. The daemon now services attached Unix sockets
in bounded slices, accepts a second client while the first is idle, and fans
out ordered journal events from independent per-client replay cursors. The
runtime integration test exercises both concurrent attach and cross-client
event delivery. The MULTICLIENT parent gate is complete. PTY writer ownership
now crosses the daemon boundary: create/attach/take/release/input/resize/
terminate commands use the daemon-owned runner, PTY state and output are
published as typed events, and disconnect releases the writer lease. Real
daemon integration covers a shell PTY and epoch-guarded input/resize. The next
active split routes the tmux-style keyboard prefix commands to real session and
layout effects. The prefix state machine itself is complete: Ctrl+B timeout,
double-prefix literal input, typed command mapping, and visible footer/help
documentation are covered without intercepting dialogs or editors. Session
create/selection/writer-control/detach routes are complete; the next active
layout-model gate supplies real split/close/focus targets for the remaining
prefix commands.
The client-local pane layout model is now typed and independent of daemon
state, with stable IDs, split axes, bounded ratios, focus/close operations, and
narrow-terminal collapse behavior covered by model tests. Split-pane rendering
now shows daemon PTY sessions with focused borders and viewer labels. Prefix
split and pane-close actions mutate only client-local layout; existing daemon
session metadata remains authoritative and session termination still follows
its separate confirmation path. The complete keyboard-prefix parent gate is
now verified; typed crossterm mouse focus/scroll routing is complete and widget
row/dialog/scrollbar/terminal interaction remains queued. Client-local
PaneLayout persistence and safe reconnect restoration are now verified; local
Unix-socket SSH reattachment is also verified and security hardening is the
next independent eligible gate.
The interactive runtime attach/poll/detach path is verified for typed daemon
effects and UI daemon-health rendering; the parent client-architecture gate is
next.
Service capability inspection now runs in the daemon with bounded PR/hash/
signature metadata and process diagnostics; release and utility runners remain
the next split gate.
The local SSH-style disconnect/reconnect acceptance test now proves a dropped
client does not terminate the daemon and a fresh same-host Unix-socket client
can reattach. Security hardening is verified: runtime directories and sockets
are private and canonical, peer UIDs are authenticated, stale socket identity
is checked, and untrusted project profiles/shell-command fields are rejected.
Resource limits are now explicit in the versioned daemon handshake and enforced
for clients, PTY sessions, dimensions, scrollback, and output, with journal
and terminal-emulator bounds retained. Daemon health telemetry is now a typed,
periodic protocol event with uptime, BitBake, client/job/PTY counts, queue
pressure, optional resident memory, and recovery phase. Daemon/session status UI
now renders those health values, instance identity, warnings, and PTY/session
state in the persistent header with Ratatui TestBackend coverage. Typed daemon
CLI management is the next active gate.
Typed daemon CLI management is now available: lifecycle start/status/stop/
restart, interactive attach, session listing/availability checks, and explicit
`--force` session termination use versioned IPC commands. Daemon integration
coverage now verifies live startup, handshake limits, reconnect cursors,
multi-client fanout, dropped clients, PTY ownership, persistence, and recovery.
Real Unix PTY integration now verifies a shell prompt, typed input, resize,
writer lease routing, and process lifecycle, with detach/reattach, cancellation,
scrollback, and terminal-emulator coverage in the PTY unit suite. SSH-style
disconnect/reconnect testing is verified with a local controlled fixture. Daemon
restart/recovery acceptance now reloads persisted metadata, reconnects clients,
marks unrecoverable jobs/PTYs Lost, and exposes explicit host-reboot relaunch
intent. The daemon now owns a bounded `ProcessBackend` BitBake build supervisor
and the `yoctui daemon build <target>` command; status reconnects show durable
job lifecycle and exit codes after the initiating client detaches. The real
Poky acceptance script clones a fresh workspace, initializes a private build
directory, starts the daemon, submits a build, reconnects for status, and
cleans up. With the host namespace prerequisite enabled, the final real Poky
scarthgap `core-image-minimal` run passed setup, detached submission, repeated
reconnect checks, and terminal daemon job reporting. All 4567 tasks succeeded,
including kernel and image creation; 3648 tasks were reused from the shared
cache. The harness prefers system host tools, clears inherited pyenv internal
hook state, allows four hours, and reclaims disposable workdirs with Poky's
standard `rm_work` class. Live daemon/Poky compatibility is verified by this
run. The next independent one-Rust product gate is verified: daemon/client remain modes of
the single Rust package and tests guard against Electron/browser drift. The
mouse runtime interaction gate is now verified with typed dialog, workspace,
Navigator, Inspector, and PTY session routing plus integration/TestBackend
coverage. Dragging split separators now resizes the validated client-local pane
tree with keyboard-equivalent bounds and persistence. Keyboard/mouse parity now
has explicit specification and TestBackend coverage; every core route keeps a
keyboard path and meaningful mouse path. Real Poky validation, collision-safe
daemon fixtures, responsive PTY delivery, and Configuration rendering pass; the
the bounded terminal lifecycle probe passes repeatedly and cannot hang the gate.
Daemon and attach documentation now covers direct/service lifecycle, client
attach/detach, SSH reconnect, PTY/session management, security/resource limits,
host reboot guarantees, troubleshooting, and the verified live-Poky validation
script. The documentation portion of the parent daemon gate is complete.
High-volume BitBake output now exercises byte-aware journal eviction: the
daemon drops only the oldest bounded log records before rejecting a snapshot,
preventing a long build from taking down the daemon when its frame limit is
reached.
Server-side IPC now also treats BrokenPipe, connection-reset, and bounded write
timeouts as client-local disconnects, preventing a slow status client from
terminating the daemon while its BitBake worker continues.
Lifecycle/status clients now allow bounded multi-megabyte snapshots several
seconds to complete while BitBake is emitting logs; short probes remain
bounded, but no longer report a healthy daemon as unavailable under load.
The server uses independent deadlines: short per-client read slices keep PTY and
job supervisor events responsive, while multi-second writes retain that large
snapshot tolerance.
The live acceptance script now preserves actionable cooker-log diagnostics
when a real BitBake build fails before its temporary workspace is removed.
Its reconnect/status probe is deliberately paced so high-volume task events do
not compete with the daemon's bounded snapshot writer.
The default live-build timeout is four hours because an uncached Poky image can
spend substantial time compiling native prerequisites in a constrained CI host.
The disposable live build enables Poky's standard `rm_work` class so completed
recipe workdirs cannot crowd out the final kernel and image tasks; downloads,
sstate, package data, and deploy artifacts remain available to the build.
The default attach snapshot now retains 512 recent records, preserving a useful
bounded history without repeatedly serializing the full high-volume log stream.

## M18 — Yocto release capability compatibility

Status: in progress.

M18 is now required. The initial audit found authoritative workspace identity
split between bridge/protocol/model fields, generic daemon transport and
BitBake-server capability enums, and independent utility-specific capability
types. It found no centralized environment identity, behavior catalog,
generation-correlated snapshot, or shared daemon-owned availability source.
The current Python bridge also selects broad legacy/modern adapter families
from the BitBake major version, while several utility inspectors treat a host
executable as the primary availability signal. M18 replaces these disconnected
assumptions with direct probes, conservative centralized fallback inference,
typed daemon/protocol/model state, dynamic UI gating, and exact live evidence.

`COMPAT-SPEC-001` is the active task. No release is newly claimed supported by
this governance change; existing fixture and live evidence retain their stated
scope.

`COMPAT-SPEC-001` is complete. The normative contract now distinguishes each
authoritative identity source from weak diagnostic hints, defines direct probe
and fallback precedence, release/support/degradation policy, all five feature
states, catalog alternatives, daemon ownership/cache invalidation, runtime UI
revalidation, and exact deterministic versus live evidence requirements. No
minimum supported series or latest supported stable is claimed before the two
required M18 live gates. `COMPAT-ENV-ID-001` is active.

`COMPAT-ENV-ID-001` is complete. The model now has a pure, serializable,
bounded `YoctoEnvironmentIdentity` whose fields independently retain Unknown or
an authoritative detected value/source. It covers BitBake, OE-Core/Poky,
DISTRO/MACHINE, configured layer series, canonical build/source roots,
initialized-environment tools, backend, and protocol. Normalization rejects
wrong authority, unsafe paths/text, oversized or empty detected inventories,
and conflicting duplicate tools/layers; six tests include partial, invalid,
duplicate, and mixed-series environments. `COMPAT-CAP-MODEL-001` is active.

`COMPAT-CAP-MODEL-001` is complete. The centralized model defines 48 stable
behavior IDs with no release-number identity, all five required availability
states, bounded typed reason/evidence records (including evidence polarity and
shell-free argv), and a normalized non-zero-generation snapshot tied to an
exact environment. Available/limited and unavailable states require positive
and negative evidence respectively; absent and uncertain records fail closed.
Five focused model tests and warnings-denied Clippy pass.
`COMPAT-CATALOG-001` is active.

`COMPAT-CATALOG-001` is complete. Versioned catalog v1 contains exactly one
typed entry for every behavior ID, including tool/command/option,
metadata/API/artifact, safe probe, preferred implementation, explicit fallback
selector, advisory release-boundary field, and exact default reason. Catalog
validation rejects incomplete, duplicate, unsafe, and selector-less data; four
focused tests and warnings-denied model Clippy pass. `COMPAT-PROBE-001` is
active.

`COMPAT-PROBE-001` is complete. The bitbake crate now evaluates typed,
non-mutating executable/version/help/option and metadata/backend/protocol/
artifact/configuration probes only in a context that exactly matches the
normalized environment. External probes use reconstructed environment, exact
argv, deadlines, bounded streams, process groups, and timeout cleanup. Missing
behavior is negative evidence; unsafe/stale, spawn/read, timeout, and truncation
remain inconclusive. Five fake-process/context tests and Clippy pass.
`COMPAT-VERSION-001` is active.

`COMPAT-VERSION-001` is complete. One numeric parser and catalog-declared map
now own all fallback version comparisons. Direct positive/negative evidence
wins; conflict, malformed/missing, undeclared, pre-map, BitBake 2.19+, and 3.x
remain Unknown. The only initial rule selects legacy Tinfoil for
`1.46..<2.0` or modern Tinfoil for `2.0..<2.19`, always with limitations and
official-source evidence, never as a release support claim. Five tests and
Clippy pass. `COMPAT-UNKNOWN-001` is active.

`COMPAT-UNKNOWN-001` is complete. A centralized resolver now preserves an
unfamiliar environment and emits all catalog records: positive-only direct
evidence enables exactly that behavior, negative-only is Unavailable, conflict
is Unknown, and absent/inconclusive behavior remains Unknown unless its bounded
catalog fallback applies. Synthetic BitBake 99.0 tests cover every case and
prove the closed historical map is not inherited. `COMPAT-OLD-001` is active.

`COMPAT-OLD-001` is complete. Snapshots now derive Full, Degraded, or
Diagnostic mode from the five capability states, without release-name allowlist
or global failure. A synthetic BitBake 1.52 environment preserves positively
probed workspace behavior, selects the limited legacy Tinfoil fallback,
disables absent Devtool upgrade, and retains a complete snapshot. A no-action
snapshot remains valid Diagnostic. No minimum release is claimed before live
evidence. `COMPAT-CACHE-001` is active.

`COMPAT-CACHE-001` is complete. Exact keys include normalized build/source,
BitBake/tool, layer/backend/protocol identity, workspace and daemon-workspace
identity, and bounded SHA-256 initialized-environment/layer/build configuration
digests. The cache holds one environment, reuses only an exact key, clears and
advances generation on change/invalidation, and rejects stale, mismatched,
oversized, and overflow state. Seven focused tests and Clippy pass.
`COMPAT-PROTOCOL-001` is active.

`COMPAT-PROTOCOL-001` is complete. Compatibility schema v1 now carries bounded
authoritative environment identity, stable capability IDs, all five states,
reasons, evidence, selected implementation, and a non-zero inner generation in
attach snapshots and complete replacement events. Validation precedes journal
mutation and rejects malformed, oversized, duplicate, contradictory, and stale
data; unknown future wire values decode fail-closed. Four focused protocol
tests and the full workspace compile pass. `COMPAT-DAEMON-001` is active.

`COMPAT-DAEMON-001` is complete. `DaemonCompatibilityCoordinator` now owns the
one exact-key cache, typed catalog probing, centralized resolution, selected
implementations, generation tickets, invalidation, and stale-result rejection.
Normalized compatibility state is daemon-global, maps once into validated wire
data, and is shared identically by attach/reconnect clients through the journal
and complete replacement events; clients perform no inference. Five focused
model/CLI/app tests and warnings-denied Clippy pass. When no authoritative
initialized context is available the daemon keeps compatibility absent/Unknown
instead of inspecting an unrelated host PATH. The command audit adds distinct
server status, start, and stop capabilities so one verified option cannot
authorize another operation. `COMPAT-BITBAKE-CMD-001` is
complete. `BitBakeCommandPlanner` now validates the exact daemon generation and
build environment, enabled behavior capability, and selected implementation
before returning shell-free argv. `ProcessBackend`, `BitBakeCliControl`, and
`SignatureAdapter` use it for build/task flags, graph generation,
environment/getvar alternatives, signature tools, and server control. Missing,
disabled, stale, environment-mismatched, and implementation-mismatched state
fails before spawn; tests prove old/new getvar forms never cross and an
unavailable build creates no process. `COMPAT-BITBAKE-API-001` is active.

`COMPAT-BITBAKE-API-001` is complete. The typed API authority validates exact
environment/generation state, enabled behavior, selected implementation, and a
coherent Tinfoil family. The additive bridge hello sends that bounded offer;
the initialized bridge directly negotiates callable behavior and returns only
the confirmed subset at the same generation. Backend operations reject stale,
unoffered, command-fallback, mixed-family, or unnegotiated support before a
command is sent. The Python bridge has no BitBake major-version switch, so
synthetic future behavior can be positively enabled while absent older APIs
degrade independently. Distinct IDs now cover recipe dependencies, sources,
metadata, and layer relationships. `COMPAT-DEVTOOL-001` is complete. The
catalog now contains 53 independently probed behavior records, including
Devtool status, edit-recipe, modify, update-recipe, finish, deploy-target,
undeploy-target, reset, and upgrade. `DevtoolCommandPlanner` requires the
exact initialized-environment executable, build directory, snapshot generation,
enabled capability, and selected implementation before returning shell-free
argv. Status inspection, PTY edit previews, local jobs, and daemon-owned jobs
all use that boundary and fail closed with the retained probe reason when the
snapshot is absent, stale, or unavailable. Focused bitbake/app/daemon tests,
all 215 bitbake tests, and workspace check pass. `COMPAT-RECIPETOOL-001` is
complete. A distinct 54th record probes `create --outfile` independently from
the create subcommand. Closed typed create/appendfile operations and
`RecipetoolCommandPlanner` require the exact environment, generation,
initialized executable, complete required capability set, and selected
implementations before returning argv. The utility menu carries these IDs and
app availability changes directly with the snapshot and exact reason. Focused
old/new, exact argv, stale, cross-subcommand, unavailable, and zero-spawn tests
pass with workspace check and Clippy. `COMPAT-LAYERS-001` is complete. The
57-record catalog distinguishes
show-layers, create-layer, create-layer with `--add-layer`, add-layer, and
remove-layer, while inventory and relationships retain separate negotiated API
records. Closed operations and `BitBakeLayersCommandPlanner` require exact
environment/generation/tool/implementation authority. Utility and app action
availability comes from that same snapshot; older command surfaces preserve
read-only/create behavior while absent mutations remain disabled with exact
reasons and zero spawn. Focused tests, workspace check, model validation, and
Clippy pass. `COMPAT-PKGDATA-001` is complete. The 62-record catalog
independently covers generated pkgdata and the exact list-pkgs, package-info,
list-pkg-files, and read-value command/options beside lookup-pkg/find-path.
`PackageDataAdapter` requires snapshot generation/environment, initialized tool
identity, artifact evidence, and selected implementation before command
creation; recursive host scanning is removed. Missing tool, missing generated
data, unavailable command, valid empty result, and command failure remain
distinct. Adapter, app, CLI, workspace, and Clippy gates pass.
`COMPAT-UTILITIES-001` is complete. A typed 19-family inventory covers every
registered utility executable and derives all five required utility states
from the daemon capability snapshot while retaining exact reasons. Unprobed
families remain Unknown, internal workers are intentionally unsupported, and
host PATH is never evidence. The generic utility command authority requires
the exact generation, build directory, initialized tool, behavior capability,
and selected implementation before shell-free argv construction. Catalog,
partial/unavailable, absent/stale/unknown, implementation-selection, workspace
check, and warnings-denied Clippy gates pass.

The broad workspace gate has been split before implementation into three
coherent children: complete behavior/catalog inventory, pure model projection
and revalidation, and protocol/app/runtime authority enforcement. The original
`COMPAT-WORKSPACE-001` remains the parent acceptance gate.
`COMPAT-WORKSPACE-CATALOG-001` is complete. The compiler-checked inventory
maps every Screen, 25 logical destinations, and every Effect/nested operation
to local behavior, daemon probing, or exact all-of/any-of requirements. The
catalog now has 76 behavior records: distinct SDK publish/native, test-family,
QA-task, buildhistory comparison, sstate, PR-service management, build-compare,
and Git-archive capabilities replace implicit workspace-local assumptions.
Typed test builds retain their family; local cancellation remains safe after
capability unloading. Focused model/catalog/utility tests, workspace check, and
warnings-denied model Clippy pass.

`COMPAT-WORKSPACE-MODEL-001` is complete. One pure authority projects all five
states, exact all-of/any-of failures, and selected implementations. Snapshot
replacement is monotonic and conflict-safe; invalidation fails closed. Every
dialog is classified, capability loss closes only unsafe environment dialogs
with restored focus/reason, and selections/local cancellation remain valid.
The capability-aware reducer boundary rolls back preparation and emits no
unavailable effect. Six focused tests, workspace check, and model Clippy pass.
`COMPAT-WORKSPACE-APP-001` is complete. Valid daemon wire snapshots convert
once into normalized model authority; malformed or unknown wire data fails
closed, stale replacements cannot displace newer authority, and disconnect or
absent data invalidates support without replacing presentation. Every
interactive daemon/local action and effect-follow-up route now crosses the same
capability-aware reducer. Client startup and post-inventory capability probes
were removed, so daemon-owned probe effects cannot spawn in a client.
Unavailable actions retain exact reasons and produce no process/job effect.
Focused app/CLI tests, all workspace tests, and warnings-denied Clippy pass.
`COMPAT-WORKSPACE-001` is complete: its closed catalog, pure model authority,
wire/client lifecycle, runtime gating, daemon-probe suppression, and exact
no-spawn rejection all pass aggregate app/model, workspace, bridge, roadmap,
and Clippy gates. The broad UI gate is split before implementation into typed
presentation state, the responsive Environment/Compatibility inspector, and
cross-workspace visible action gating. `COMPAT-UI-001` remains the parent
acceptance gate. `COMPAT-UI-MODEL-001` is active.

`COMPAT-UI-MODEL-001` is complete. One bounded client-local state projects the
current daemon authority into authoritative identity, generation/mode, exact
five-state counts, stable sorted capability rows, reasons/requirements,
limitations, selected implementations, and typed evidence. Filters, bounded
search, and selection reconcile by stable capability ID across replacement and
invalidation. Disconnected, synchronizing, absent-current, and stale authority
remain explicitly unavailable, and reusable action presentation consumes the
same workspace projection without a second cache. Focused model/app tests,
workspace check, and warnings-denied Clippy pass.
`COMPAT-UI-INSPECTOR-001` is complete. Compatibility is now a first-class,
client-local Navigator and command-palette destination. Its responsive view
renders only the typed daemon-authority projection: authoritative identity,
generation/mode, all five state counts, stable filtered/searched capability
rows, exact reasons and requirements, limitations, selected implementations,
and bounded typed evidence. Wide, medium-overlay, narrow-pane, absent-authority,
every-theme, no-color, long-content, replacement, and boundary-size coverage
passes without changing the canonical Tasks golden or F1-F10 rail.
The broad visible-action gate was split before implementation into a closed
typed action-surface catalog, global Navigator/palette/footer rendering,
workspace/Inspector rendering, and dialog rendering/enforcement. The original
`COMPAT-UI-ACTIONS-001` remains the parent acceptance gate.
`COMPAT-UI-ACTION-CATALOG-001` is complete. One typed classifier maps every
destination, command-palette command, contextual typed effect, and dialog to
the existing exhaustive requirement model. Client-local, inspectable, and
capability-gated activation are distinct: navigation stays reachable under
missing authority, operations fail closed, and local/owned cancellation stays
usable. All five states, exact reasons, and selected implementations survive
projection without a renderer cache or version policy. Focused exhaustive,
absent, limited-fallback, effect, and dialog tests plus model Clippy pass.
`COMPAT-UI-NAV-ACTIONS-001` is complete. Navigator rows now carry concise
five-state markers from centralized destination requirements while remaining
selectable; focused Navigator Inspector/footer text gives the exact reason and
selected fallback. Command-palette rows merge local prerequisites with typed
command compatibility, keep navigation discoverable, and reject unavailable
operations before dialog preparation. Live replacement/invalidation updates
both surfaces without a local cache. Focused tests, full model/app/UI suites,
literal golden, PTY snapshots, and workspace Clippy pass.
`COMPAT-UI-WORKSPACE-ACTIONS-001` is complete. A stable contextual action
inventory covers all 25 destinations and each useful environment-backed
operation, with explicit local/open/cancel entries. Inspectors render exact
five-state rows, reasons, and selected implementations from the live authority;
the Tasks action list gains state without changing the canonical literal
golden. Invalidation removes stale fallback text immediately, runtime denials
remain no-spawn, and local/owned operations remain usable. Focused and full
model/app/UI suites, PTY snapshots, and workspace Clippy pass.
`COMPAT-UI-DIALOG-ACTIONS-001` is complete. Every environment-backed dialog
uses the centralized dialog requirement and a compact two-line compatibility
rail to show its five-state result, exact reason, limitation, selected
implementation, and confirmation availability without obscuring responsive
dialog content. Limited actions remain confirmable, denied confirmations emit
no effect, and newer snapshot invalidation closes unsafe dialogs with restored
focus and the exact reason. Local dialogs and owned cancellation remain usable.
Focused tests, full model/app/UI suites, PTY snapshots, and workspace Clippy
pass. `COMPAT-UI-ACTIONS-001` is complete. Aggregate UI/app acceptance proves
all action surfaces consume the same live projection, agree on fallback
implementations, fail closed after invalidation, and retain local behavior.
Focused aggregate tests, PTY snapshots, roadmap validation, and workspace
Clippy pass. `COMPAT-UI-001` is complete: the typed projection, responsive
Compatibility workspace, all visible action surfaces, exact explanations,
live replacement, and local action behavior pass nine aggregate UI and six app
tests plus PTY snapshots. `COMPAT-DOCTOR-001` is complete. Doctor now consumes
the validated attached daemon snapshot and emits exact human or bounded JSON
identity/state/evidence diagnostics; unavailable and malformed authority fail
closed, while release support remains honestly Unknown pending live matrix
evidence. Focused CLI/protocol tests and workspace Clippy pass.
`COMPAT-MATRIX-001` is complete. The exact development observation remains
Partially tested, minimum/latest support are explicitly unclaimed, and all six
policy labels are defined. Structure validation rejects ambiguous rows and
requires non-fixture latest/older evidence before any support claim. The docs
gate now validates bridge protocol lifecycle and Doctor JSON without bypassing
daemon capability authority. Documentation and structure checks pass.
`COMPAT-TEST-FIXTURES-001` is complete. Five reusable, explicitly fixture-only
policy roles carry typed identities, direct observations, and exact expected
states/implementations through the production catalog and resolver. Complete
snapshots prove legacy/modern boundaries, direct override, closed future
fallback, and positive-only future enablement. Focused workspace tests and
Clippy pass. `COMPAT-TEST-CMDS-001` is complete. One command authority derived
from each shared fixture carries explicit direct command-surface evidence and
selected implementation IDs without using release labels as availability.
Ten focused tests inspect exact typed argv for BitBake native/fallback forms,
Devtool, Recipetool, bitbake-layers, and pkgdata; absent options and subcommands
fail before process construction and no external command is executed. The
workspace all-feature command gate passes. `COMPAT-TEST-UI-001` is complete.
Dedicated dynamic suites run five model,
five app, and four TestBackend cases covering monotonic live replacement,
stale/conflicting response rejection, stable selection, immediate action and
reason replacement, unsafe dialog close/focus restoration, local-dialog
retention, invalidation, and denied-action rollback with no emitted launch.
The exact task filters and workspace Clippy pass. `COMPAT-BITBAKE-GETVAR-001`
is complete after fresh Wrynose 6.0.2 / BitBake 2.18.0 validation proved that
`bitbake --getvar` is unsupported and this release exposes the separate
`bitbake-getvar` utility. That utility is now a typed initialized-environment
identity with direct help/`--value`/`--recipe` probes; its exact executable and
argv are selected together, while `bitbake -e` remains available only as an
explicit capability-backed fallback. Direct BitBake, server-control, and
signature consumers reject configured/authorized executable disagreement.
Focused old, modern, absent-tool, stale-generation, and exact-executable tests,
the full all-feature workspace suite, Clippy, bridge tests, and roadmap gate
pass. `COMPAT-DAEMON-RUNTIME-001` is complete. The production daemon now
requires an initialized `BUILDDIR`, derives a bounded direct identity and
configuration fingerprint, deduplicates and runs safe catalog probes, installs
generation one before attach, and shares that authority with the journal,
Doctor, Devtool, and BitBake supervisors. Missing authority fails closed before
BitBake spawn, while unprobed backend/task inventories remain inconclusive
rather than false negative. Same-directory utility symlinks retain their exact
invocation name, and per-supervisor event budgets keep daemon IPC responsive
under live parse traffic. Fake initialized-environment tests cover exact
identity, core layer series, publication, no-PATH-only authority, and bounded
invalid input.

Fresh official Wrynose 6.0.2 / BitBake 2.18.0 validation confirmed exact Poky,
DISTRO, MACHINE, COREBASE/core-series, layer roots, utility identities,
`bitbake-getvar` implementation, Doctor transport, bridge negotiation, a real
`base-files` build reaching native parse/task events, and responsive status
during event bursts. `COMPAT-PROBE-AGGREGATION-001` is complete. Catalog version
two now rejects duplicate required probes, and the centralized resolver enables
only a complete all-positive probe set. A negative requirement disables,
contradiction conflicts, and absent/partial/inconclusive required evidence stays
Unknown without an implementation. Static fallback is considered only after a
complete all-inconclusive direct set. Multi-generation fixtures enumerate every
required observation, and focused resolver/version/future tests pass. A fresh
live Doctor snapshot confirmed the timed-out `devtool upgrade --help` is Unknown
with `evidence.incomplete`, while fully positive `bitbake-getvar` remains
Available and the fully inconclusive backend build probe selects its explicit
limited adapter fallback. `COMPAT-BITBAKE-CANCEL-RUNTIME-001` is complete.
Bridge-native event iteration and daemon event publication are independently
bounded, queued cancellation has deterministic supervisor priority, and an
accepted BitBake shutdown becomes exactly one build-correlated terminal even
when Tinfoil omits a final event. Late native records are discarded. Fake
bridge and real-backend daemon flood tests pass. A live official Wrynose 6.0.2 /
BitBake 2.18.0 `core-image-minimal` run was cancelled through daemon IPC at
67/4346 tasks and reached Failed in 0.257 seconds; a second client and Doctor
remained responsive, and the bridge/server child exited without force kill.
`COMPAT-BITBAKE-CANCEL-ORDER-001` is complete. Live Scarthgap 5.0.19 /
BitBake 2.8.1 exposed a distinct ordering failure: accepted cancellation
stopped the bridge/server, but thousands of already queued native records kept
the shared job Running beyond 15 seconds. Explicit cancellation now uses the
typed cooker force-shutdown operation, cleanup acknowledgement is bounded, and
a priority terminal channel invalidates pre-cancel records without replaying
them after terminal state. A 10,000-event delayed-cleanup regression passes,
and live `core-image-minimal` cancellation now reaches Failed in 0.405 seconds.
`COMPAT-LIVE-LATEST-001` is complete. Authoritative release documentation
selected official Wrynose 6.0.2 / BitBake 2.18.0 as the latest published stable
on 2026-08-19 and supplied exact OE-Core, BitBake, and meta-yocto revisions.
The daemon/Doctor identity agrees with those components; 1,922 Recipes, three
Layers, capability-authorized Configuration, Devtool and utility surfaces,
modern command options, a successful `base-files:do_listtasks` native-event
run, and the independent 0.257-second cancellation passed. Headless backends
now install the daemon snapshot and reject absent or mismatched authority before
operation. The machine-readable 90-day record and offline verifier reject
fixtures, stale dates, ambiguous commits, missing workflows, implausible
metrics, and non-ancestor Yoctui code. The exact revision is Tested, while a
supported release window remained gated on the older anchor and matrix
automation. `COMPAT-LIVE-OLDER-001` is complete. Authoritative support policy,
calendar, and release notes selected maintained Scarthgap 5.0.19 and bound the
fresh Poky checkout to Poky `bb983546`, OE-Core `2814f096`, BitBake `0880963f`
(`2.8.1`), and meta-yocto `2f749ae`. Daemon/Doctor identity and degraded reasons,
1,829 Recipes, three Layers, capability-authorized Configuration, initialized
utility surfaces, an exit-0 `base-files:do_listtasks` run with 77 typed
observations, and 0.405-second cancellation passed. The exact Tested revision
is the lower proposed support anchor; `COMPAT-LIVE-MATRIX-001` is active next.
`COMPAT-LIVE-MATRIX-001` is complete. The deterministic default validates both
live records, expiry/support policy, exact differing identities/compositions,
version ordering, and non-claiming development semantics. Explicit live roles
fetch exact official revisions into isolated roots and produce diagnostics
without rewriting tracked evidence. Fresh Scarthgap 5.0.19 monolithic Poky and
Wrynose 6.0.2 split-component runs both passed daemon identity, inventories,
configuration, dependency graph, native task events, cancellation, and typed
server teardown. `COMPAT-CI-001` is active next.
`COMPAT-CI-001` is complete. Normal CI now runs a network-free compatibility
gate across the typed model, probes/resolver/argv, app, UI, bridge, future
behavior, and offline evidence policy. A separate weekly/manual matrix runs
exact older/latest fresh roles with explicit opt-in and bounded timeout, then
always uploads role-scoped Doctor, daemon, inventory, and BitBake diagnostics.
The static CI contract rejects accidental PR live runs, missing roles/bounds/
artifacts, deterministic coverage omissions, and network operations in the fast
script. `COMPAT-DOC-001` is complete. User documentation now states the
Yocto-feature-correlated rule, compares one binary across unavailable Devtool,
native/fallback BitBake variable lookup, older/degraded, and future-unknown
environments, and points to the shared Compatibility inspector and Doctor
snapshot. The release policy names the exact Scarthgap 5.0.19 and Wrynose 6.0.2
Tested anchors, preserves the distinction from a broader Claimed-supported
window, documents the 90-day renewal rule, and keeps fixtures and optional
development runs non-claiming. `COMPAT-001` is complete. The dedicated gate now
discovers every required M18 compatibility task, retains an explicit minimum
contract, checks the product rule in README and compatibility documentation,
requires exact Claimed-supported latest and older matrix rows, and delegates to
the deterministic and strict live-evidence verifiers. The exact current support
anchors are Scarthgap 5.0.19 / BitBake 2.8.1 and Wrynose 6.0.2 / BitBake 2.18.0;
other point releases, fixtures, and development snapshots remain outside that
claim. All M18 children and the parent are complete.
