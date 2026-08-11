# Yoctui UI Specification

Status: **Authoritative product and interaction contract**

This document defines how Yoctui must look, behave, navigate, present BitBake state, and expose Yocto workflows.

The implementation agent must follow this document. It must not invent new layouts, panes, dialogs, shortcuts, focus rules, or interaction patterns without updating this file in the same commit.

---

## 1. Product goal

Yoctui is a one-stop terminal workspace for Yocto and BitBake development.

It must combine:

- workspace and layer browsing
- recipe and metadata inspection
- file preview and editor launching
- BitBake build control
- live task monitoring
- warnings and error investigation
- dependency exploration
- Devtool workflows
- configuration and provenance inspection
- package, image, SDK, testing, QA, QEMU, Wic, sstate, CVE, SPDX, and maintenance workflows

Yoctui is not a collection of unrelated screens. It is a persistent workbench with a consistent navigation, focus, dialog, and shortcut model.

BitBake remains authoritative. Yoctui presents, controls, and organizes BitBake state.

---

## 2. Persistent application shell

The normal application layout is:

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ Yoctui | Build #12 | MACHINE qemux86-64 | DISTRO poky | Target core-image-minimal       │
│ Status RUNNING | Tasks 2148/4821 | Active 12 | W 3 | E 0 | SState 86% | CPU 82% | 28m  │
├──────────────────┬─────────────────────────────────────┬──────────────────────────────────┤
│ Navigator        │ Workspace                           │ Inspector                        │
│                  │                                     │                                  │
│ Dashboard        │ Context-specific list/tree/table    │ Preview/details/live output      │
│ Layers           │                                     │                                  │
│ Recipes          │                                     │                                  │
│ Tasks            │                                     │                                  │
│ Logs             │                                     │                                  │
│ Errors           │                                     │                                  │
│ Configuration    │                                     │                                  │
│ Packages         │                                     │                                  │
│ Images           │                                     │                                  │
│ SDK              │                                     │                                  │
│ Testing          │                                     │                                  │
│ Security         │                                     │                                  │
│ QA               │                                     │                                  │
│ Devtool          │                                     │                                  │
│ QEMU / Wic       │                                     │                                  │
│ Maintenance      │                                     │                                  │
├──────────────────┴─────────────────────────────────────┴──────────────────────────────────┤
│ ? Help  F5 Build  / Search  Tab Focus  Ctrl+P Commands  e Errors  l Logs  q Quit         │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

The shell contains five persistent regions:

1. Header
2. Navigator
3. Workspace
4. Inspector
5. Shortcut/status footer

Dialogs and notifications are drawn above this shell.

---

## 3. Header

The header is always visible unless the terminal is too small.

It shows compact live build and environment state:

- application name
- active build/session number
- backend
- build status
- target
- task when applicable
- `MACHINE`
- `DISTRO`
- completed/total task count
- active task count
- warning count
- error count
- estimated sstate reuse
- elapsed time
- CPU utilization
- memory utilization when available
- build filesystem free space

The header must never horizontally panic. It progressively hides low-priority metrics on narrow terminals.

Priority order:

1. status
2. target
3. task progress
4. errors/warnings
5. machine
6. distro
7. elapsed time
8. sstate
9. CPU/memory/disk

---

## 4. Navigator

The left pane is the primary workspace navigator.

Required entries:

- Dashboard
- Layers
- Recipes
- Tasks
- Logs
- Errors
- Configuration
- Packages
- Images
- SDK
- Testing
- Security
- QA
- Devtool
- Dependencies
- QEMU / Wic
- Maintenance
- Settings

The currently active workspace is highlighted.

The navigator may show badges:

```text
Tasks          12
Errors          3
Logs          LIVE
Devtool         2
Testing       FAIL
```

Keyboard:

- `j` / `Down`: next entry
- `k` / `Up`: previous entry
- `Enter`: activate entry
- single-letter global shortcuts may jump directly to common workspaces
- `Tab`: move focus to workspace

---

## 5. Focus model

Exactly one focus target is active:

```rust
enum FocusTarget {
    Navigator,
    Workspace,
    Inspector,
    Dialog,
    CommandPalette,
}
```

Rules:

- `Tab`: next focus target
- `Shift+Tab`: previous focus target
- arrow keys affect only the focused region
- `Esc`: close dialog, cancel transient mode, or return focus outward
- dialogs trap focus until closed
- opening a dialog or command palette remembers the active pane; transitions
  between nested modal states keep that return target, and closing the final
  modal restores it
- pane navigation and workspace activation actions are ignored while modal
  focus is trapped
- exactly one typed dialog is active: the front of the retained dialog queue
- a dialog workflow may replace its active variant while preserving the
  original pane return target
- asynchronous completion arriving while a user dialog is active is queued
  and shown after that dialog closes; it never interrupts or discards input
- inactive panes remain visible but use subdued styling
- focus must be visibly obvious in every theme

No workspace may invent a conflicting focus model.

---

## 6. Workspace behavior

The center pane is the active work area.

A workspace owns:

- list/tree/table contents
- selection
- scrolling
- search query
- active filters
- sort order
- local toolbar/action availability

The inspector reflects the currently selected item.

Changing selection must update the inspector without changing focus.

Opening a significant action uses a dialog or external editor; it must not replace the persistent shell.

---

## 7. Layers workspace

The Layers workspace behaves like an IDE file explorer.

Example:

```text
meta-openembedded/
├── meta-oe/
│   ├── conf/
│   ├── classes/
│   ├── recipes-core/
│   └── recipes-support/
├── meta-networking/
│   └── recipes-connectivity/
│       └── curl/
│           ├── curl_8.10.1.bb
│           ├── curl.inc
│           └── files/
└── meta-python/
```

Required behavior:

- all configured layers are visible
- directories expand and collapse
- directory contents are loaded lazily
- directories sort before files
- hidden files can be toggled
- layer priority and compatibility are visible
- active build-related layers can be highlighted
- search filters layers, paths, and filenames
- selected files preview in the inspector
- selected directories show metadata and relationships
- open the selected layer in the in-TUI two-pane tree/editor
- open the selected layer root in the configured external editor/file manager
- refresh selected subtree
- detect modified, untracked, and generated files where Git information is available

The configured-layer inventory stays pinned above the active layer tree. It
shows each layer's priority and reported compatibility; the currently browsed
layer is selected and layers supplying the active target/tasks use the active
semantic role. Expanding a directory caches only that directory's immediate
children. Collapse never discards cached descendants, while refresh replaces
only the selected subtree listing.

Tree Git decorations are `M` for modified, `?` for untracked, `I` for
ignored/generated, and `-` when Git is unavailable. Hidden entries are loaded
but omitted until `.` toggles them on, so toggling does not cause a recursive
scan or lose path identity.

From the configured-layer inventory:

- `Enter`: open the selected layer's lazy metadata browser
- `e`: open the selected layer in the large in-TUI two-pane editor
- `o`: open the selected layer root in the configured external editor
- `R`: inspect authoritative layer relationships

Inside the layer browser/editor:

- `Right` / `l`: expand
- `Left` / `h`: collapse or move to parent
- `Enter`: edit a file or toggle a directory
- `e`: edit the selected file
- `r`: refresh
- `.`: toggle hidden files
- `/`: search
- `g`: Git details
- `m`: metadata view
- `d`: dependencies view
- `Esc`: return to the configured-layer inventory

The tree must not eagerly scan the entire Yocto source tree.

---

## 8. Inspector

The right pane is context-sensitive.

Supported inspector modes include:

- file preview
- metadata summary
- effective variable values
- provenance
- recipe dependencies
- task dependencies
- reverse dependencies
- layer relationships
- selected task live log
- error details
- package details
- artifact details
- Git status/diff
- test result details

Tabs may appear at the top of the inspector:

```text
[Preview] [Metadata] [Dependencies] [History] [Git]
```

Inspector rules:

- read-only by default
- scroll independently
- preserve position per selected item when practical
- show full path
- show file size and modification status
- line numbers for text preview
- syntax highlighting when practical
- binary files show metadata, not raw terminal garbage
- large files are streamed or truncated safely
- show a clear message when preview is unavailable

Layer text previews are limited to 64 KiB, show line numbers, and retain
BitBake/Markdown syntax styling. A truncation banner identifies bounded
previews. Invalid UTF-8 or NUL-containing files are treated as binary and show
metadata only. Preview responses carry their source path, so a late response
cannot replace the newly selected file's Inspector.

Layer and Devtool source workflows use a large two-pane in-TUI editor: the
left pane retains the bounded lazy tree and the right pane shows the selected
syntax-aware preview or editable file. `Ctrl+S` saves an edited file and
`Esc` returns to the prior workspace. Ordinary Inspector previews remain
read-only.

Where a workspace exposes its external-open action (`o` in Layers), Yoctui
suspends the terminal and launches `$EDITOR` or the configured editor. After
the editor exits, Yoctui restores the terminal and refreshes the affected
file, Git state, and metadata.

---

## 9. Dashboard workspace

The Dashboard provides the high-level current state.

Required sections:

- current build summary
- recent builds
- active tasks
- recent warnings and errors
- sstate prediction/reuse
- system telemetry
- common actions
- last artifacts
- environment diagnostics

The dashboard must be useful both when idle and during a build.

Idle actions:

- start build
- inspect workspace
- choose recent target
- open doctor results
- run sstate readiness check
- open recent artifact
- resume previous filters/workspace

Running actions:

- open Tasks
- open Logs
- open Errors
- cancel build
- inspect current recipe/task
- view queue statistics

---

## 10. Build dialog

`F5` or `B` opens the image build-options dialog. A lower-case `b` remains a
contextual selected-target action, including the selected-recipe build in
Recipes.

Example:

```text
┌─ Start Build ───────────────────────────────────────────────┐
│ Targets       [ core-image-minimal                       ]  │
│ Task          [ default                                  ]  │
│ Machine       [ qemux86-64                               ]  │
│ Backend       [ bridge                                   ]  │
│ Options       [ ] continue   [ ] force   [ ] verbose       │
│ Preflight     SState estimate: 86%   Disk: 312 GB free      │
│                                                             │
│              [ Start ]   [ Cancel ]                         │
└─────────────────────────────────────────────────────────────┘
```

Requirements:

- targets support history and completion
- invalid targets are rejected before execution where possible
- machine and distro changes are explicit
- advanced BitBake options are separated from the normal path
- preflight can run `oe-check-sstate`
- destructive or unusual flags require confirmation
- starting a build creates a background job
- the UI stays interactive
- default post-start behavior is to focus Tasks while keeping all workspaces accessible

---

## 11. Live Tasks workspace

The Tasks workspace is the main live build monitor.

Example:

```text
Overall  [██████████████████████████▊             ]  67%  3214/4821
Rate     148 tasks/min      Active 16      Waiting 530      Elapsed 22:14

Recipe                 Task                 Elapsed    State       Progress
openssl                do_compile           00:14.2    RUNNING     ▸▸▸▸▸▸▸▸
linux-yocto             do_compile_kernel    03:51.0    RUNNING     ▰▱▰▱▰▱▰▱
busybox                 do_package           00:02.4    RUNNING     ▹▹▹▹▹▹▹▹
glibc                   do_package_write_rpm 00:31.8    RUNNING     ▱▰▱▰▱▰▱▰
```

### Animated progress

Task progress must look active and responsive.

Rules:

- indeterminate tasks use animated patterns
- animations advance on UI ticks, not backend event rate
- different phases may use different patterns
- the overall build bar uses real completed/total task progress
- animation must not imply false numeric completion
- animation speed is configurable
- default animation should feel fast and energetic
- animation must remain readable over SSH and low-refresh terminals
- reduced motion freezes unknown-progress activity to the stable word `active`
- unknown progress is labeled `progress unknown` and never rendered as `0%`
- determinate, completed, and failed rows never show indeterminate animation

Suggested frames:

```text
▸▸▸▸▸▸▸▸
▹▸▸▸▸▸▸▸
▹▹▸▸▸▸▸▸
▹▹▹▸▸▸▸▸
```

or:

```text
▰▱▱▱▱▱▱▱
▱▰▱▱▱▱▱▱
▱▱▰▱▱▱▱▱
```

Do not redraw the whole application unnecessarily just to animate bars.

Task selection updates the inspector with:

- live log
- task metadata
- recipe
- PID where available
- start time
- elapsed time
- dependencies
- source log path
- cancellation/termination state

Filters:

- active
- waiting
- completed
- failed
- recipe
- task
- worker
- duration threshold

Task controls:

- `↑`/`↓` or `k`/`j` moves the bounded task selection
- `f` cycles all, active, waiting, completed, and failed state filters
- `F` selects the recipe, task, or worker text-filter field
- `/` edits the selected text filter; `Enter` or `Esc` finishes editing
- `d` cycles all, one-second, ten-second, and one-minute duration thresholds

When BitBake reports a total without individual queued-task identities, the
workspace shows one honest aggregate waiting row. It must not invent recipe,
task, worker, or timing metadata for those queued tasks. The Inspector labels
unavailable task fields explicitly.

---

## 12. BitBake output consumption

The UI renderer must never parse raw BitBake text.

All backend input flows through typed normalized events:

```rust
enum BackendEvent {
    BuildStarted,
    ParseStarted,
    ParseProgress,
    TaskQueued,
    TaskStarted,
    TaskProgress,
    TaskCompleted,
    TaskFailed,
    Warning,
    Error,
    Log,
    BuildCompleted,
    CancellationRequested,
    CancellationAcknowledged,
    BackendDisconnected,
}
```

Pipeline:

```text
BitBake / bridge / process backend
                ↓
       normalization adapter
                ↓
        bounded event channel
                ↓
             reducer
                ↓
            AppState
                ↓
            renderer
```

Raw process output is retained only as an optional diagnostic/log source.

### Output priority and backpressure

Never discard:

- task failure
- build failure
- warnings
- errors
- cancellation status
- backend disconnect
- final build result

Ordinary informational logs may be evicted or coalesced when limits are reached.

High-frequency progress events may be coalesced by task identity.

Every dropped or coalesced event count must be observable.

---

## 13. Logs workspace

Required features:

- live follow
- pause/resume follow
- wrap toggle
- vertical scrolling
- horizontal scrolling when wrap is off
- incremental text search
- next/previous match
- severity filter
- recipe filter
- task filter
- selected build filter
- source-path display
- open source log in editor
- copy selected line/details
- bounded retention and eviction counters

The selected log entry appears in the inspector with full multiline content and metadata.

Controls:

- `↑`/`↓` or `k`/`j` selects an older/newer visible entry and pauses follow
- `f` toggles live follow; resuming selects the newest matching entry
- `w` toggles wrap; horizontal offset resets when wrap is enabled
- `←`/`→` scrolls horizontally only while wrap is disabled
- `/` starts incremental search; `Enter` or `Esc` finishes it
- `n`/`N` selects the next/previous search match
- `s`, `R`, `T`, and `B` cycle severity, recipe, task, and build filters
- `o` opens the selected source path in the configured editor
- `C` copies structured selected-entry details when a supported clipboard tool
  is available

Retention prefers warnings, errors, cancellation records, disconnects, and
final results over ordinary informational entries. Repeated adjacent ordinary
entries may be coalesced. Evicted warning/error counts and the coalesced count
remain visible. If only protected records exceed a configured limit, eviction
is still bounded and explicitly counted.

---

## 14. Errors workspace

Warnings and errors are structured records, not merely colored log lines.

List columns:

- time
- severity
- recipe
- task
- summary
- build session

Inspector:

- complete multiline message
- normalized category
- source log
- relevant source path
- task and recipe
- event metadata
- suggested actions
- related warning/error entries
- jump to matching log context
- open source file/log in editor

Build completion behavior:

- zero errors: success notification
- warnings only: warning summary notification
- errors: prominent failure notification with direct action to open Errors
- cancelled build: cancellation summary distinct from build failure
- pressing `Enter` on a failure notification opens the selected error

Each retained warning/error has a stable session identity and typed category,
summary, event metadata, and suggested actions. `Enter` on a selected
diagnostic opens that exact retained entry in Logs without overwriting the
user's existing query or filters. `o` opens its source path when present.
`↑`/`↓` or `k`/`j` changes the bounded diagnostic selection.

The completion dialog uses the same outcome distinctions as notifications.
For failures with retained diagnostics, `Enter` opens Errors while any other
key dismisses the dialog. Backend loss is an actionable failure diagnostic,
not an empty error state.

---

## 15. Recipes workspace

Required:

- recipe name
- preferred/resolved version
- providing layer
- append count
- workspace/devtool status
- build status
- search and filtering
- selected recipe details
- dependencies and reverse dependencies
- tasks
- source paths
- patches
- package outputs
- history where available

The recipe inventory obtains resolved version, provider file/layer, and append
count from BitBake's parsed provider tables. `Enter` lazily refreshes the
selected recipe's authoritative tasks, metadata sources/appends, patch URIs,
and package outputs. Fields the backend cannot supply, including workspace,
per-recipe build status, or history, remain explicitly unavailable until a
typed source provides them. Inventory refresh preserves selection by recipe
name and removes details for recipes that disappeared.

Recipe rows show resolved and preferred version separately, provider layer,
append count, workspace/Devtool status, and build status. Search matches recipe
name, both versions, layer, and provider path; the selected absolute recipe
identity is shared by rendering and every action. Typed active/completed tasks
and the current build may enrich build status without parsing logs. The
Inspector distinguishes not loaded, loading, failed, unavailable, available
empty, and populated details, and always lists dependencies, reverse
dependencies, tasks, metadata sources, patches, package outputs, and history.

Recipe actions use dialogs:

- build
- force task
- clean
- cleansstate
- devshell
- menuconfig
- diffconfig
- diffsigs
- open recipe
- open task log
- Devtool modify
- Devtool update-recipe
- Devtool finish
- Devtool reset
- Devtool deploy-target
- patch review
- CVE check
- SPDX generation

The default recipe build opens a typed confirmation immediately. Standard
task routes for `clean`, `cleansstate`, `devshell`, `menuconfig`,
`diffconfig`, and `diffsigs` are available only when the selected recipe's
authoritative task metadata advertises them. `f` opens a focus-trapping task
picker populated from that same metadata; its force toggle cannot be supplied
as raw text. Every confirmation previews the exact target, task, and force
intent before it creates a persistent build job. The process and bridge
backends preserve the typed target/task/force fields when invoking BitBake.

Destructive, forced, and otherwise unusual task execution requires explicit
confirmation. Empty or stale selection, unavailable task metadata, unsupported
tasks, malformed task names, and a duplicate active build stay inert and
produce an actionable notification.

`z` remains the ordinary confirmed BitBake `diffsigs` task shortcut when that
task is advertised by recipe metadata. `Z` opens the separate signature
inspection workflow and never launches a BitBake build task. It first opens a
focus-trapping task picker populated only from the exact selected recipe's
authoritative task metadata. `Up`/`Down` moves, `Enter` requests the chosen
recipe/task dump and opens the Signatures child workspace, and `Esc` restores
the prior Recipes pane. Missing, stale, empty, or invalid metadata leaves the
workspace unchanged and produces an actionable notice.

The Signatures child workspace is not a duplicate Navigator entry. Its header
always names the exact recipe and task. The record pane lists each bounded
historical signature by hash and authoritative path and marks the selected
comparison sides as `1`, `2`, or both. The detail pane shows the selected
record's base/task hashes, typed variables, and task dependencies. The
comparison pane groups typed base-hash, changed-value, dependency, and
unavailable-field differences. Signature artifacts are read-only and are not
opened as source text.

Signature workspace keys are:

- `Up`/`Down`: select an exact historical identity
- `1` / `2`: assign the selected identity to that comparison side
- `c`: compare two complete, distinct sides
- `r`: refresh the exact recipe/task dump
- `e`: open the selected recipe provider through the normal validated editor
- `Esc`: return to Recipes while idle, or request cancellation while a dump or
  comparison is running

Dump and comparison work runs asynchronously and remains cancellable so
terminal drawing and navigation continue. Results are correlated to the exact
target or comparison request; a late result cannot replace newer state. The
workspace renders not-loaded, loading, available-empty, available, partial,
and failed states explicitly. Partial views retain usable typed data and show
every bounded limitation. Missing tools/files, path rejection, nonzero exits,
timeouts, and cancellation are visible outcomes rather than empty success.

At wide sizes, records occupy the left pane and selected detail/comparison
content occupies the right. Narrow layouts stack these regions; very small
layouts show a compact state and shortcut summary without panicking. The
context footer is `↑↓ Select  1/2 Sides  C Compare  R Refresh  E Provider  Esc
Back/Cancel`, dimming actions that are unavailable.

`e` opens the selected recipe's provider file using the configured editor.
`o` opens its retained task log directly or, when multiple authoritative log
paths remain, opens a task/state/path picker. `p` does the same for
BitBake-resolved local patch paths. Remote or unresolved patch URIs are shown
as unavailable and are never converted into guessed filesystem paths. The
pickers trap focus; `Up`/`Down` selects, `Enter` opens, and `Esc` returns to the
exact prior pane. A path that disappeared after discovery, a missing editor,
or a non-zero editor exit produces an actionable notification after the
terminal is restored.

The same Recipes workspace exposes typed Devtool modify, update-recipe,
finish, deploy-target, and reset routes for the absolute selected recipe
identity. Existing preview and confirmation requirements remain in force.
`t` refreshes Devtool status directly, and `Enter` refreshes it after recipe
metadata. The Recipes and Devtool Inspector shows executable availability,
workspace membership and absolute source path, and Git branch, head,
clean/dirty state, and modified/untracked/conflicted counts. Missing
executables, a missing workspace source directory, a non-Git source, failed
commands, and malformed output are distinct states.

The Inspector derives every Devtool action's enabled state and disabled reason
from that typed status. A recipe outside the workspace may be modified but
cannot be updated, finished, deployed, or reset. A missing workspace source
may be reset but not edited or published. An existing source opens directly
for editing with `d`; finish remains disabled until Git reports a commit and
no modified, untracked, or conflicted files. Status responses are stored by
recipe name plus absolute provider path, so a response for a prior selection
does not replace the current recipe's state.

Confirmed Devtool operations run as cancellable background jobs without
suspending the terminal. The selected recipe Inspector shows the latest
matching Devtool job, retained stdout/stderr identity, explicit truncation,
status, and terminal outcome. Navigation remains available while the process
runs, and the retained job remains visible after leaving and returning to the
recipe. One Devtool operation may be active at a time; a duplicate request is
inert with an explanation. `c` cancels the active Devtool job before any
independent BitBake job, with graceful and forced cancellation distinguished.
Missing tools, start failure, nonzero exit, cancellation failure, and runner
loss remain distinct outcomes.

`d` never guesses Devtool eligibility. When the exact selected recipe identity
has no authoritative status, it directs the user to refresh with `t`; missing
Devtool, status errors, and invalid workspace states remain disabled with their
typed reason. A recipe reported outside the workspace opens a focus-trapping
confirmation that previews the exact `devtool modify <recipe>` operation and
provider path. A recipe already reported in the workspace opens its
authoritative source tree directly.

Successful modify completion refreshes the original recipe identity even when
the user navigated elsewhere while the job ran. Only a refreshed absolute
workspace source path may open the large two-pane workspace editor. Refresh
failure, absent membership, a missing source directory, or file scan/load
failure leaves the successful job and refreshed status visible with a
recoverable notification. The editor retains the source tree on the left and
syntax-aware selected-file preview/editing on the right. `Ctrl+S` saves;
`Ctrl+B` refuses dirty content, otherwise closes the editor and opens the
existing exact `bitbake <recipe>` confirmation. It never routes to an image
build.

`u` follows the same exact-identity rule for update-recipe. Unknown status,
missing Devtool, status errors, non-membership, and missing workspace sources
remain inert with the typed reason. An eligible recipe opens a focus-trapping
confirmation showing both `devtool update-recipe <recipe>` and the absolute
provider path. Successful persistent completion refreshes the original recipe
identity even after navigation and reports the refreshed workspace state.
Nonzero exit, cancellation, runner loss, or status-refresh failure leaves the
retained job output and prior actionable context intact.

`F` requires the exact authoritative recipe identity, a present workspace
source, and Git status with a commit and no modified, untracked, or conflicted
files. It opens a focus-trapping picker containing only configured layers whose
reported paths are absolute; the recipe's provider layer is selected when it
is available. `Up`/`Down` changes the layer, `Enter` previews, and `Esc`
cancels. There is no free-text finish destination.

The finish confirmation shows the exact
`devtool finish <recipe> <native-destination>` intent, provider path,
configured layer name, and destination. Confirmation revalidates both
eligibility and current configured-layer membership, so stale, relative, or
unconfigured paths stay inert. Successful persistent completion refreshes the
original identity; removal from the Devtool workspace is a valid refreshed
state. Command, cancellation, runner, and refresh failures retain the durable
job and actionable prior state.

`P` requires the exact authoritative recipe identity and a present workspace
source before opening target entry. The draft retains that identity while the
user enters exactly one non-option target value; empty values, whitespace,
control characters, and option-like values are rejected before confirmation.
The focus-trapping confirmation shows the exact
`devtool deploy-target <recipe> <target>` intent, absolute provider path, and
target, then revalidates eligibility and target syntax immediately before
execution.

Deploy-target runs as the same persistent cancellable Devtool job, preserving
stream identity, graceful/forced cancellation, nonzero failure, runner loss,
and navigation retention. A successful terminal event refreshes only the
original recipe identity. Process or refresh failure leaves prior
authoritative status and durable job context available.

`D` is the destructive reset route. It requires the exact authoritative recipe
identity and either a present workspace source or an explicitly reported
missing workspace directory. Non-membership, missing Devtool, status errors,
relative source paths, and stale source changes remain inert. The
focus-trapping confirmation shows the exact `devtool reset <recipe>` intent,
absolute provider path, and workspace source path that will be removed, then
revalidates all three immediately before execution.

Reset uses a persistent cancellable Devtool job with retained stream identity
and distinct graceful/forced cancellation, nonzero failure, and runner-loss
outcomes. Successful completion refreshes the original identity; `not in
workspace` is the expected terminal state. A refresh that still reports a
workspace or that fails remains explicit without erasing the durable job.

`V` starts a selected-recipe CVE check only when authoritative metadata
reports `do_cve_check`; `X` starts SPDX generation only when it reports
`do_create_spdx`. Each route opens the existing typed confirmation with the
exact `cve_check` or `create_spdx` BitBake task and then creates a distinct,
cancellable persistent QA job. The Recipes Inspector shows both capability
reasons and the latest matching QA job's status, progress, warning/error
counts, retained outcome, and reported artifacts. A successful task with no
typed artifact path says `none reported`; Yoctui never guesses a CVE report or
SPDX output directory from console text.

Unavailable actions are shown disabled with an explanation in the footer or inspector.

---

## 16. Dependencies workspace

This workspace integrates:

- `bitbake -g`
- `oe-depends-dot`
- server-supplied dependency information
- recipe dependencies
- task dependencies
- reverse dependencies
- build-order paths
- “Why is this built?” path tracing

Layout:

- center: navigable typed dependency rows
- inspector: selected node details, reverse/outgoing context, limitations, and
  path explanation

Graph rendering must degrade gracefully in terminals. A tree/path view is mandatory; a visual graph is optional.

The center rows come only from normalized `DependencyGraphState` nodes and use
their deterministic model order. Each row shows recipe or task kind, its exact
identity, and incoming/outgoing edge counts. Build, runtime, and task edge
families are named in the Inspector; widgets never infer an edge from names,
logs, or provider paths. `↑`/`↓` or `k`/`j` changes the selected typed identity.
Selection survives refresh when that identity remains, otherwise it returns to
the graph root (or the first reported node if no root node exists).

The Inspector always shows:

- exact root and selected recipe/task identity
- absolute provider and task-log paths when supplied, otherwise `unavailable`
- sorted incoming edges as reverse-dependency context
- sorted outgoing edges as dependency context
- every adapter limitation
- one deterministic shortest root-to-selection why-built path, bounded to 64
  edges and 4,096 visited nodes

The path is rendered as ordered typed identities and edge kinds. Root
selection says `root selected`; a disconnected node says `unreachable from
root`; exhausting either bound says `path limit reached`. Cycles never repeat
nodes or hang. Long identities, paths, and limitation text wrap or truncate
within the active responsive pane.

Workspace shortcuts:

- `↑`/`↓` or `k`/`j`: select a typed node
- `Enter`: open the selected node's owning recipe in Recipes when that exact
  recipe exists in the authoritative inventory
- `o`: open only the selected node's absolute typed provider path
- `L`: open only the selected task's absolute typed log path
- `r`: refresh the same typed graph root
- `Tab`/`Shift+Tab`: use the global pane focus cycle
- `Esc`: return to Dashboard through the global action

Missing inventory entries, provider paths, or task logs leave the action inert
and show an exact notification. `o` never guesses a recipe file from layer
layout, and `L` never searches console text. Recipe nodes may expose a provider
but never a task log unless the backend explicitly supplies one.

State presentation is explicit:

- not loaded: explain that Recipes `g` starts dependency inspection
- loading: show the exact requested root and no stale graph rows
- available-empty: show the root and `no dependency edges reported`
- available: show the typed rows and Inspector
- partial: show the typed rows plus every limitation
- failed: show the requested root and exact failure; stale graph data is not
  presented as current

Wide mode uses the persistent Inspector. Medium Inspector overlay and narrow
pane switching retain the same selected identity and content. The global
too-small view remains authoritative below 80×24. All supported breakpoint
sizes must render empty, partial, cyclic, deeply bounded, and long-path data
without panic.

---

## 17. Configuration workspace

Read-only by default.

Required:

- effective value
- unexpanded value where available
- global or recipe-specific scope
- provenance chain
- overrides
- appends/prepends/removals
- defining file and line when available
- search
- copy value
- open defining source
- compare values between recipes or configurations where supported

The workspace begins with the backend's effective global-variable summary and
is read-only. `↑`/`↓` or `k`/`j` moves through the filtered, sorted variables;
`/` edits the shared metadata search; and `Enter` lazily refreshes the selected
global `VariableIdentity`. Selection remains attached to the same identity
when a refreshed summary still contains it.

The Inspector distinguishes not loaded, loading, failed, available-empty, and
populated detail. Populated detail shows scope, effective and unexpanded
values, provenance, active overrides, and every typed set/append/prepend/remove
operation with its defining file, line, and value when supplied. Results and
errors are keyed by variable name plus optional recipe scope. A scoped or stale
response cannot complete or replace a selected global request.

Wide mode may show the same selected detail in the persistent Inspector while
the workspace keeps its table and detail region. Medium and narrow layouts
retain the selected semantic pane and wrap long values and operation paths.
Empty searches, missing metadata, partial operations, and backend failures
remain explicit and never cause a panic.

`C` copies the selected detail's effective value and `U` copies its unexpanded
value through the shared typed clipboard effect. These actions use only loaded
detail for the exact selected `VariableIdentity`; the summary-table value is
never used as a fallback. Missing selection, loading, failed or not-yet-loaded
detail, and an absent value keep the action inert and show the exact disabled
reason in the Inspector. Clipboard-tool failures remain actionable
notifications from the CLI boundary.

`o` opens defining sources only from the loaded detail's typed operations.
When exactly one distinct file/line source exists it opens directly. Multiple
sources open a focus-trapping picker showing operation, path, and line;
`↑`/`↓` or `k`/`j` selects, `Enter` opens, and `Esc` restores the prior pane.
Relative operation paths are resolved against the active build directory at
the effect boundary, and parent traversal is rejected. Missing selection,
loading, failed or not-loaded detail, no file-backed operation, a stale
choice, a missing build directory, a disappeared file, or an editor failure
stays inert and produces an exact explanation. Summary provenance remains
visible context but is never parsed as the source-action authority.

`s` opens a focus-trapping scope picker for the selected variable. Its first
row is global scope and the remaining rows are sorted, deduplicated recipe
names from the authoritative workspace inventory. `↑`/`↓` or `k`/`j` selects,
`Enter` activates the scope and starts a typed detail request, and `Esc`
restores the prior pane. An empty recipe inventory leaves global scope
available and says that no recipe scopes were reported.

The global summary table remains global; the active optional recipe scope is
stored separately and combines with the selected variable name to form the
Inspector's `VariableIdentity`. Loading, error, and loaded records remain
independent per scope. Copy and defining-source actions automatically follow
the active identity and never fall back to another scope. Recipe-inventory
refresh preserves a still-present scope and returns to global if that recipe
disappears; responses for a prior scope remain cached but cannot alter the
active Inspector or its action availability.

`c` opens a focus-trapping, read-only comparison between the selected
variable's exact loaded global detail and active recipe-scoped detail.
Effective and unexpanded fields each carry typed `equal`, `different`, or
`unavailable` outcomes and show both values; an absent value is never treated
as equal. `Enter` or `Esc` closes and restores the prior pane. Comparison stays
disabled with a precise reason when no recipe scope is active, a scope
disappeared, either identity is loading/failed/not loaded, or no variable is
selected. Long values wrap safely in every responsive mode.

Configuration remains read-only unless all edit prerequisites are satisfied.
The initial explicit allowlist is `MACHINE` and `DISTRO`; every other variable
shows a read-only reason. Editing is global-scope only and requires the exact
selected identity's loaded effective value plus an active build directory.
Recipe-scoped, loading, failed, not-loaded, and absent values remain inert.

`E` opens a focus-trapping value editor prefilled from that authoritative
effective value. `Enter` validates a single-line value, escaping quotes and
backslashes, then opens a separate confirmation dialog. Newline and other
control-character injection is rejected. The confirmation shows the exact
destination `build/conf/local.conf` and exact quoted assignment. Its `Enter`
revalidates the typed request, replaces the exact active variable assignment
or appends it when absent through a permission-preserving atomic rename, then
refreshes that exact global identity from BitBake. A write failure leaves the
file and prior detail untouched; a refresh failure retains the prior detail
and reports that the write succeeded. `Esc` from either dialog performs no
write and restores the exact prior pane. Editing never writes before the
second, preview-confirming `Enter`.

No silent edits.

---

## 18. Packages, images, SDK, and test results

### Packages

Integrate `oe-pkgdata-util`.

`Packages` is a first-class Navigator destination after `Recipes`. Activating
it starts a correlated background inventory query when package data has not
been loaded. The persistent shell continues drawing and accepting navigation
while the query runs. `R` refreshes the inventory, and `c` requests
cancellation of the active package inventory or detail query. Leaving the
workspace does not discard a pending result; request generations prevent a
stale result from replacing newer state.

The Workspace has explicit not-loaded, loading, available-empty, available,
partial, and failed presentations. Missing generated `tmp/pkgdata` explains
that a target must complete `do_package`; it is not presented as an empty
package set. Partial results render their bounded limitations. Raw
`oe-pkgdata-util` text is never displayed or parsed by a widget.

The package list shows:

- package name
- recipe
- version
- size
- license
- image membership

Unavailable fields render as `unavailable`, distinct from an available empty
value. Wide mode uses aligned package, recipe, version, size, and license
columns. Medium mode uses a compact two-line row. Narrow mode uses one
identity-first row and the standard Navigator / Workspace / Inspector
switcher. All modes preserve selection by exact runtime-package identity.

`Up`/`Down` or `j`/`k` select packages. `/` traps input in package search;
typing filters package, recipe, version, license, and authoritative provider
fields case-insensitively. `Backspace` edits and `Enter` or `Esc` leaves search
without clearing it. `Enter` lazily requests the exact selected package
detail.

The Inspector shows the selected identity and summary plus separate Files,
Runtime dependencies, Reverse dependencies, and Image membership sections.
Every section distinguishes unavailable from available-empty. Detail loading,
partial, and failed states remain visible while the inventory list stays
usable. `D` switches the active dependency section between runtime and reverse,
`[`/`]` selects the previous/next dependency, and `d` follows the selected
dependency only when that exact identity exists in the current inventory.
`u` returns through the bounded package navigation history.

`o` opens the authoritative owning recipe in the Recipes workspace when its
exact identity exists there. `e` opens the authoritative absolute provider
path in the selected editor. Missing recipe identity, provider path, package
detail, dependency list, or inventory identity leaves the action inert and
shows a contextual explanation; Yoctui never fabricates a provider.

The Packages footer is:

```text
↑/↓ select | Enter detail | / search | R refresh | D dep kind | [/ ] dep | d follow | u back | o recipe | e provider | c cancel
```

### Images

Images combines buildable image recipe targets with deployed artifacts without
presenting one as evidence for the other. The Workspace header shows the
effective `MACHINE`, selected/current build target, artifact search query, and
count. `i` keeps the existing image-recipe picker. Entering Images starts one
artifact request only while state is not loaded; `R` explicitly refreshes and
`c` requests cancellation.

Wide mode uses the persistent Navigator, an artifact table in Workspace, and
the selected artifact in Inspector. Medium mode keeps the artifact table and
uses the shared Inspector overlay. Narrow mode uses the shared visible-pane
switcher; both list and Inspector remain reachable. The shared too-small
message applies below the supported boundary. Selection is keyed by exact
machine/image/path identity and survives refresh when that identity remains.

The artifact table shows image target, adapter-classified kind, file name,
size, and timestamp. It renders distinct not-loaded, loading,
available-empty, available, partial, failed, and no-search-match states.
Partial state keeps valid rows usable and shows the limitation count. A
selected artifact never changes merely because an asynchronous stale result
arrives.

The Inspector shows:

- exact machine and image target
- adapter-classified artifact kind
- absolute deployed path and authoritative deploy directory
- byte size and modification timestamp
- checksum algorithm, digest, and checksum source
- manifest paths
- license paths
- SPDX/SBOM paths
- Wic-related paths
- every typed scan/model limitation

Unavailable fields render `unavailable`; available empty collections render
`none`. Widgets never classify names, parse checksum text, derive paths from
logs, or treat a missing field as empty.

`b` opens the normal build confirmation for the selected artifact's exact
image target. When no artifact is selected it preserves the existing
current-image behavior. `o` opens the selected artifact path. `m`, `l`, `s`,
and `w` open the first exact typed manifest, license, SPDX/SBOM, or Wic path
respectively. Missing selection or typed path leaves the action inert and
shows a stable explanation. All opens use the configured editor and normal
terminal restoration.

The Images footer is:

```text
↑/↓ select | / search | R refresh | c cancel | b build | i image picker | o artifact | m manifest | l license | s SPDX | w Wic
```

Search edits only while search mode is active; Enter or Esc finishes editing.
Dialogs trap focus. Light, dark, monochrome/no-color, and every responsive
breakpoint preserve state meaning with labels and attributes rather than color
alone.

### SDK

`SDK` is a first-class Navigator destination after `Images`. It uses the
persistent shell and shared background-job lifecycle; leaving the workspace
never discards an active SDK build, scan, publication, native-tool operation,
or terminal result. Once the typed model destination is present but before an
adapter scan has run, the Workspace explicitly reports that SDK artifact
rendering/acquisition is pending rather than borrowing Images state.

The Workspace header shows the exact active `MACHINE`, `DISTRO`, selected image
recipe target, and authoritative SDK deploy root. `i` opens the existing
machine-aware image target picker. `s` previews a standard SDK build as the
typed BitBake task `do_populate_sdk`; `E` previews an extensible SDK build as
`do_populate_sdk_ext`. `t` and `T` preview `do_testsdk` and `do_testsdkext`
respectively. Each preview names the exact image target, task, machine, and
distro before `Enter` starts the existing managed BitBake build. `Esc` closes
without starting. Testing launches remain visible here for SDK context, while
the Testing workspace owns unified result comparison and export.

After a successful populate task, and on explicit `R`, Yoctui scans only the
canonical absolute SDK deploy root reported by typed BitBake configuration.
The inventory is generation-correlated and distinguishes not loaded, loading,
available empty, available, partial, and failed. The adapter, not widgets,
classifies bounded regular non-symlink installers, checksum files, manifests,
and other SDK artifacts. Rows preserve exact path identity and show SDK kind,
host/target tuple when authoritative, byte size, modification time, and
publication state. Missing metadata renders `unavailable`; no filename pattern
is presented as authoritative metadata.

The Inspector shows the selected exact artifact identity, related checksums and
manifests, lifecycle/result/output for its originating SDK operation, every
scan limitation, and any validated extracted-SDK root. `↑`/`↓` changes exact
selection. `o` opens the selected regular artifact. `/` searches typed identity
and metadata. `c` requests cancellation of the active SDK-owned operation.

`P` opens a publication destination dialog only for a selected publishable
installer. The next overlay shows the exact indexed, shell-free
`oe-publish-sdk` argument vector, installer identity, and absolute destination;
publication never begins without explicit confirmation and never guesses an
overwrite policy. Output, nonzero failure, cancellation, and loss remain in
SDK history.

`n` opens the native-tool dialog. The user selects either the active build
workspace or a validated extracted SDK root, enters a bounded recipe/tool
identity and bounded native arguments, then confirms the exact indexed
`oe-find-native-sysroot` or `oe-run-native` vector. Tools execute directly
without a shell. Extracted roots must be canonical directories with an exact
adapter-validated environment setup identity; widgets never source scripts or
derive environment variables. Environment changes are confined to the managed
child process and never mutate the Yoctui process.

The native-tool dialog initially selects `Mode`. `↑`/`↓` moves between Mode,
Workspace, Recipe, Tool, and Arguments without wrapping. `Enter` on Mode, or
`←`/`→` while Mode is selected, switches between find-sysroot and run-native.
`Enter` on a text field begins editing and `Enter` finishes it; `Esc` closes
the dialog even while editing. An empty Workspace means the active build.
Arguments are bounded text whose ASCII-whitespace-delimited tokens become the
exact shell-free argument vector. `p` validates and opens the indexed preview.
The Tool field is explicitly not applicable in find-sysroot mode.

All SDK dialogs trap focus and remain usable at 80×24. Responsive modes follow
the shared Navigator/Workspace/Inspector rules, all lifecycle and selection
meaning survives no-color mode, and long paths/arguments are bounded and
wrapped. The SDK footer is:

```text
↑/↓ select | i image | s standard | E extensible | t testsdk | T testsdkext | R refresh | P publish | n native | o open | c cancel
```

At 90 columns and below, the one-line footer compacts these labels to
`↑↓ i:image s/E:SDK t/T:test R:scan P:publish n:native o:open c:cancel`.
Every SDK shortcut remains visible; only its label is abbreviated.

### Testing

`Testing` is a first-class Navigator destination after `SDK`. It owns unified
test launch context and structured results without duplicating SDK launch
state or exposing an arbitrary command textbox. The persistent Header shows
the active Yocto release, build directory, `MACHINE`, `DISTRO`, selected image,
aggregate active-job state, CPU utilization, and build-filesystem space as on
the other workspaces.

The Testing Workspace has three typed views selected by `Tab`: `Launches`,
`Results`, and `Comparison`. Each view retains its own exact selection and
search state across navigation and resize. The Inspector follows the selected
row. Before capability inspection completes, the Workspace says that testing
capability and result acquisition are pending; missing tools, unsupported
tasks, missing configuration, empty results, partial data, adapter failure,
and cancellation are distinct states.

#### Launches

The Launches view contains these fixed typed families:

- OE selftest (`oe-selftest`)
- BitBake selftest (`bitbake-selftest`)
- image runtime (`do_testimage`)
- standard SDK (`do_testsdk`)
- extensible SDK (`do_testsdkext`)
- package tests (`ptest`)

The Inspector explains the selected family's authoritative executable or
BitBake task, current image/machine/distro binding, configuration
prerequisites, selector, and exact disabled reason. It never claims that a
test suite exists based only on a display name.

`Enter` or `r` opens the selected family's launch dialog. OE selftest has an
`All` or `Selected` scope, a bounded exact test identifier for Selected, and
parallelism from 1 through 256. All maps to `oe-selftest -a`; Selected maps to
`oe-selftest -r <identity>`, and optional parallelism maps to an indexed `-j`
argument. BitBake selftest has an optional bounded exact unittest identity,
typed verbose choice, and typed skip-network choice. Skip-network is a
child-only `BB_SKIP_NETTESTS=yes` environment entry, never a mutation of the
Yoctui process.

Image runtime, standard SDK, and extensible SDK launches bind the current
exact image target and use `BuildRequest` tasks `testimage`, `testsdk`, and
`testsdkext` through the existing managed BitBake coordinator. `i` opens the
existing machine-aware image picker. The SDK workspace may launch the two SDK
tasks contextually, but Testing is the owner of unified result inspection and
comparison.

Ptest execution is available only when typed BitBake configuration confirms
that the selected image includes ptest support and its authoritative
`TEST_SUITES` includes the ptest runtime suite. It then uses that exact
configured `do_testimage` request. Yoctui does not silently edit
`DISTRO_FEATURES`, `EXTRA_IMAGE_FEATURES`, or `TEST_SUITES`, does not SSH to a
guessed target, and does not substitute a host-side `ptest-runner` command.
When prerequisites are absent, the row remains visible with the exact
configuration explanation; imported ptest results remain supported.

Every launch dialog begins on its first editable field. `↑`/`↓` or `k`/`j`
moves without wrapping, `←`/`→` or `h`/`l` changes typed choices, and `Enter`
begins or finishes bounded text input. `p` validates and opens a second
confirmation overlay containing either the complete indexed shell-free
argument/environment vector or the exact image/task/machine/distro
`BuildRequest`. Only `Enter` in that preview starts execution. `Esc` closes
either step without starting, including while editing. Dialogs trap focus and
restore the previous pane.

#### Execution lifecycle

Every confirmed launch creates one stable `TestSession` associated with a
shared background job. BitBake task families reuse the one existing build
coordinator; selftests use one CLI-owned shell-free test runner. A session
retains family, exact selector, active image/configuration identity, start and
finish timestamps, bounded stdout/stderr, truncation/drop counts, exit status,
structured result paths emitted by the adapter, and a distinct queued,
starting, running, cancelling, succeeded, failed, cancelled, timed-out, or
lost outcome. Navigating away never discards it.

`x` opens cancellation confirmation for the active Testing-owned operation.
Cancellation rejection restores the running state with a visible reason.
Testing cancellation never targets an unrelated SDK, QEMU, Wic, Devtool,
metadata, or artifact operation. Successful execution triggers import only
for exact regular non-symlink result paths returned by the adapter. Success
with no structured result is reported honestly and does not fabricate cases.

#### Results

`resulttool` capability is inspected independently. Results are local
`testresults.json` records: each identity contains its canonical absolute
path, byte size, modification timestamp, and bounded content fingerprint.
Yoctui initially indexes only exact result paths emitted by managed sessions
or explicitly selected through the import dialog. `I` opens that dialog for a
normalized absolute regular file or directory. `R` rescans only retained
validated roots. It never recursively searches the whole build tree or
interprets a task log as structured results.

The adapter owns JSON/resulttool parsing and normalization. The model receives
bounded typed runs, suites, cases, status, duration, metadata, and exact
related log paths. Case status is `passed`, `failed`, `skipped`, `error`, or
`unknown`; missing status/duration/metadata remains unavailable. Inventory and
import state distinguishes not loaded, loading, available empty, available,
partial with limitations, failed, cancelled, timed out, and worker loss.
Malformed or oversized records are skipped with visible bounded limitations,
not converted into successful empty data.

The Results Workspace shows one row per exact result identity with family,
machine/image when authoritative, revision when present, pass/fail/skip/error
counts, duration, and timestamp. `↑`/`↓` selects; `/` searches typed identity
and metadata; `Enter` drills into that result's bounded suite/case rows and
`Esc` returns one level. The Inspector shows exact metadata, counts, selected
case status/duration, limitations, originating session, and related logs. `o`
opens the exact selected result JSON and `l` opens the selected case's exact
regular log through the configured editor. Missing paths leave the action
inert with a stable explanation.

#### Comparison and JUnit export

`c` opens a two-step picker for distinct exact baseline and candidate result
identities. The preview names both paths, fingerprints, machine/image/revision
metadata, and the exact indexed resulttool operation before `Enter` starts it.
Results are correlated to both identities; changing or removing either input
makes the response stale.

Comparison normalizes cases by exact suite and case identity:

- `regression`: present in both, passed or skipped in the baseline, failed or
  errored in the candidate
- `new failure`: absent in the baseline and failed or errored in the candidate
- `new pass`: failed or errored in the baseline and passed in the candidate
- `removed`: present only in the baseline
- `unchanged/other`: every remaining status transition

The Comparison view shows category totals and selectable exact case
transitions. Search and Inspector behavior match Results, including exact
metadata/log paths and visible partial limitations. No filename, ordering, or
free-form resulttool text determines a category in widgets.

`J` in Results opens JUnit export for the selected exact result. The
destination must be an absolute `.xml` path beneath an existing canonical
directory, and the destination itself must not already exist. A second overlay
shows the selected result fingerprint, destination, and exact indexed
shell-free resulttool vector. `Enter` exports; `Esc` cancels. Success,
nonzero failure, cancellation, timeout, stale input, and worker loss remain
distinct, and Yoctui never guesses an overwrite policy.

All Testing views and dialogs remain usable at 80×24. Wide mode uses
Navigator/Workspace/Inspector; medium mode uses the normal Inspector overlay;
narrow mode uses the visible pane switcher. Long identities, metadata, output,
and limitations are bounded and wrapped. Themes and no-color mode preserve
selection, status, category, and failure meaning with labels and attributes.

The full Testing footer is:

```text
Tab view | ↑/↓ select | Enter open | r run | i image | / search | I import | R refresh | c compare | J JUnit | o result | l log | x cancel
```

At 90 columns and below it compacts to:

```text
Tab:view ↑↓ Enter r:run i:image /:find I/R:results c:compare J:JUnit o/l:open x:cancel
```

### Security

`Security` is a first-class Navigator destination after `Testing`. It owns
typed CVE findings, package-to-upstream mappings, and SPDX/SBOM report
inspection. It does not replace the contextual `V` CVE and `X` SPDX shortcuts
in Recipes or the exact SPDX artifact link in Images; those routes use the same
typed operation and report state.

The Security Workspace has `CVEs` and `SBOM` views selected by `Tab`. Each view
retains its exact selection and search state across navigation and resize.
Entering Security requests one generation-correlated capability inspection and
one bounded report scan only when their state is not loaded. `R` refreshes the
current view from the same authoritative roots. `c` requests cancellation of
the exact active Security-owned operation. Capability pending, unavailable,
available, partial, failed, cancelled, timed out, and worker-loss states remain
distinct.

#### Capability and scope

The capability result records the active Yocto release, build directory,
`MACHINE`, `DISTRO`, selected recipe/image identities, authoritative recipe
task names, exact report roots, and optional package-mapping tool identity.
Support is derived only from typed BitBake metadata and canonical filesystem
inspection. In particular:

- a CVE check is enabled only for a target that reports `do_cve_check`;
- recipe SBOM generation uses the exact reported task, such as
  `do_create_recipe_sbom` on a current release or `do_create_spdx` on a legacy
  release;
- an image SBOM is treated as generated by a normal image build only when
  typed configuration and deploy metadata report that behavior;
- package mapping is enabled only when an exact canonical
  `cve-check-map-pkgs` capability is available;
- missing classes, tasks, tools, variables, or roots remain visible with the
  exact disabled reason.

Yoctui never edits `INHERIT`, enables a configuration fragment, guesses a task
from the release label, or treats the existence of a similarly named file as
proof that a workflow is configured.

`s` cycles exact scope between selected recipe and selected image where both
are authoritative. `i` opens the existing machine-aware image picker. `e`
opens the selected recipe's exact provider through the configured editor.
Missing selection or metadata keeps the corresponding action inert with a
stable explanation.

#### CVE view

The CVE Workspace shows one row per exact normalized finding with CVE ID,
recipe/package, upstream product and version when reported, mapped status,
severity/score when reported, and source report identity. Status is typed as
`vulnerable`, `patched`, `ignored`, `not affected`, or `unknown`; adapter
status text that is not in the supported map remains `unknown` with a visible
limitation. Widgets never infer vulnerability from color, severity, a patch
filename, or arbitrary log text.

`↑`/`↓` selects a finding. `/` searches typed CVE ID, recipe/package, product,
version, status, and summary fields. `f` cycles `all`, `vulnerable`, `patched`,
`ignored`, `not affected`, and `unknown`. The Inspector shows the exact report
path and fingerprint, complete typed status, score/vector/link fields when
reported, package-to-upstream mapping, source metadata, related recipe
identity, and every bounded limitation. Missing fields render `unavailable`;
an available empty report renders `no findings`.

`v` opens the selected exact CVE URL only when the adapter supplied an
`https` URL. `o` opens the exact source report and `e` opens the exact recipe
provider. These paths and URLs are never reconstructed from a CVE ID or
display name.

`V` opens a two-step CVE-check confirmation for the current exact scope. The
preview shows target, exact `cve_check` task, machine/distro context, indexed
shell-free BitBake request, and report roots that will be rescanned. Only
`Enter` in the preview starts the existing managed BitBake coordinator; `Esc`
closes without starting. Successful completion triggers a correlated bounded
rescan. Success with no new typed report remains successful-with-no-report and
does not fabricate findings.

`M` opens package mapping only when the canonical tool and required exact
input identity are available. Its preview shows the complete indexed
shell-free vector and input report identity. Mapping runs in one independent
Security runner, streams bounded output, and refreshes the same CVE generation
on success. It cannot be used as a free-form command launcher.

#### SBOM view

The SBOM Workspace shows one row per exact canonical SPDX document or archive
with scope, recipe/image identity when authoritative, SPDX version, document
name, byte size, modification time, and adapter-classified kind. Exact
identity contains absolute path, byte size, modification timestamp, and a
bounded content fingerprint. Supported regular JSON documents are summarized
into typed document namespace, data license, creators, packages, files,
relationships, checksums, and external references. Archives and unsupported
SPDX schemas remain openable exact artifacts with explicit summary
limitations; widgets do not parse raw JSON or archives.

`↑`/`↓` selects a document and `/` searches its typed identity and metadata.
The Inspector shows exact root/path identity, scope, schema/version, document
metadata, bounded component counts and selected component detail when
available, related image/recipe, checksums, and all limitations. `Enter`
drills from a summarized document into its bounded package/component rows;
`Esc` returns one level. `o` opens the exact document or archive using the
configured editor. `e` opens an exact related provider when supplied.

`X` opens SBOM generation for the current exact scope. Recipe generation uses
the exact capability-reported task. Image generation previews either the exact
reported SBOM task or an ordinary image build whose typed configuration says
it emits the selected SBOM class output. The confirmation shows target, task
or image build, machine/distro context, expected authoritative scan roots, and
the complete indexed request. It never silently chooses between legacy and
current task names. `Enter` starts the managed BitBake coordinator and `Esc`
cancels the dialog. A successful build triggers a correlated rescan; no
reported artifact remains an explicit empty result.

#### Report acquisition and lifecycle

Security scans only canonical absolute roots supplied by typed BitBake
configuration, exact artifact paths emitted by managed jobs, or paths chosen
through `I` import. Import accepts one normalized absolute regular
non-symlink CVE JSON/text report, SPDX JSON document, or supported archive; a
directory import is bounded to that canonical directory and never escapes or
recursively searches the build tree. Report identities are revalidated before
parse and before open.

All scans are replaceable-generation operations with bounded file count,
directory count, bytes, record count, field length, and parse time. Valid
records survive alongside malformed or oversized records as `partial` with
limitations. Empty, missing, permission-denied, stale, symlink, escape,
malformed, cancelled, timed-out, and worker-loss outcomes remain distinct.
Raw report text, JSON, filenames, process output, and BitBake logs never cross
into widgets as authority.

Security BitBake operations reuse the single managed build coordinator.
Package mapping uses at most one independently polled Security runner, and
report scanning uses one replaceable worker. Every operation retains its exact
scope, request/report generation, background-job identity, lifecycle, bounded
output, warning/error counts, result paths, and terminal outcome across
navigation. Cancellation targets only the exact Security-owned operation and
never an unrelated build, Testing, SDK, QEMU, Wic, Devtool, or metadata job.

All Security views and dialogs remain usable at 80×24. Wide mode uses the
persistent Navigator/Workspace/Inspector shell; medium and narrow modes use
the shared Inspector overlay and visible-pane switcher. Long paths, findings,
metadata, and limitations are bounded and wrapped. Every theme and no-color
mode preserves status and severity meaning with text and terminal attributes.

The full Security footer is:

```text
Tab view | ↑/↓ select | s scope | i image | / search | f status | V CVE check | M map | X SBOM | I import | R refresh | Enter details | o report | e recipe | v advisory | c cancel
```

At 90 columns and below it compacts to:

```text
Tab:view ↑↓ s:scope i:image /:find f:status V:check M:map X:SBOM I/R:data Enter o/e/v:open c:cancel
```

### QA

`QA` is a first-class Navigator destination after `Security`. It owns
capability-driven recipe, kernel, and configured-layer validation without
replacing the exact task, patch-review, or provider routes already available
in Recipes. The persistent Header shows the active Yocto release, build
directory, `MACHINE`, `DISTRO`, exact QA scope, active managed job, CPU
utilization, and build-filesystem space.

The QA Workspace has `Recipe & Kernel` and `Layer QA` views selected by
`Tab`. View, exact selection, search/filter state, session history, and
findings survive navigation and resize. Entering QA requests one correlated
capability inspection and scans only already-authoritative report roots; it
does not launch checks automatically.

#### Capability, catalog, and scope

The capability result contains the exact release/build identity, selected
recipe and provider, all eligible configured layers, canonical executable
identity when layer QA is available, and a typed check catalog. Every catalog
entry has a stable identity, family, label, exact scope, execution kind,
reported BitBake task or native vector, expected report roots, availability,
and disabled reason.

The required check families are:

- kernel configuration;
- URI and fetch metadata;
- patch application and patch metadata/status;
- license metadata and checksum;
- general recipe/package QA;
- configured-layer compatibility through `yocto-check-layer`.

Support is derived only from typed BitBake metadata, configuration, configured
layer identity, and canonical tool discovery. For example, kernel
configuration may use a reported `do_kernel_configcheck`, URI validation may
use a reported `do_checkuri`, and recipe/package or license validation may use
other exact reported tasks. These names are examples, not defaults: an entry
is runnable only when the capability supplies its exact task. Yoctui never
selects a task from the release string, treats a similarly named task as
equivalent, edits inherited classes to enable a check, or uses a free-form
command field.

In `Recipe & Kernel`, `s` cycles only exact recipe scopes supplied by the
capability. The selected recipe name and absolute provider path form one
identity; a stale provider invalidates the preview. Kernel-only entries show a
stable disabled reason for non-kernel scopes. In `Layer QA`, configured layer
name plus canonical root path form the identity. Only layers in the active
typed layer inventory are selectable; Yoctui never scans arbitrary
directories or reconstructs a layer path from its name.

#### Recipe and kernel checks

The Workspace shows one row per catalog entry with family, exact task,
availability, latest status, warning/error counts, and report availability.
`Up`/`Down` selects a check. `/` searches typed label, family, task, recipe,
provider, result, and finding fields. `f` cycles `all`, `failed`, `warning`,
`passed`, `skipped`, and `unknown`.

`r` opens a two-step confirmation for the selected available check. The
preview shows stable operation ID, exact recipe/provider scope, complete
indexed shell-free BitBake request, expected report roots, and any known
limitations. `Enter` starts it through the existing single managed BitBake
coordinator; `Esc` closes without execution. Duplicate builds remain rejected
by that coordinator. Successful completion triggers an exact
generation-correlated report scan. Success with no authoritative report is a
successful session with `no report supplied`; widgets never derive findings
or paths from console text.

The Inspector shows the selected check's capability source, exact scope and
task, current/previous sessions, bounded output summary, exact report
identities, and every limitation. It also shows normalized findings with
stable check/finding identity, typed status (`passed`, `warning`, `failed`,
`skipped`, or `unknown`), severity when reported, bounded message, recipe,
task, source path/line when authoritative, rule/code, and suggestion when
reported. Missing fields render `unavailable`; an available empty report says
`no findings`.

`Enter` drills from a check into its bounded findings and `Esc` returns one
level. `o` opens the selected exact report, `e` opens the exact provider, and
`l` opens an exact finding source only when the adapter supplied a canonical
path. Existing Recipes patch review remains the contextual way to browse all
resolved local patches; QA source opening targets only the selected normalized
finding.

#### Layer QA

The Layer QA Workspace lists every configured layer even when
`yocto-check-layer` is missing. Rows show exact layer name/root, compatibility
metadata when available, capability state, latest session status, pass/warn/
fail/skip counts, and report availability. Search and status filters operate
only on typed rows/findings.

`r` opens a deterministic confirmation only when the capability contains a
canonical regular executable and exact vector for the selected configured
layer. The preview shows stable session identity and the complete indexed
native argument vector; there is no extra-arguments textbox. Confirmation
immediately revalidates the executable and layer root, then starts one
shell-free child in its own process group. At most one layer-QA runner is
active. Both output streams are bounded and tagged, while widgets consume
only typed runner events and normalized findings.

Layer findings use the same typed status and source fields as recipe/kernel
findings plus exact layer identity and test name. Unsupported output remains
bounded session output with a visible parsing limitation; it is never silently
promoted to a pass or failure. Nonzero exit, cancellation, timeout, duplicate
start, stale tool/layer identity, and worker loss are separate outcomes.

#### Reports, imports, lifecycle, and dialogs

`I` opens a focus-trapping import dialog for one normalized absolute canonical
regular QA report or a bounded canonical directory. Only documented adapter
formats are parsed. Imports and successful operations replace the current
report generation; `R` rescans the same exact paths. Scans are bounded by
directory/file count, total bytes, record count, field length, and time, refuse
symlinks and escapes, and preserve valid findings beside malformed data as
`partial`.

Exact report identity contains canonical path, byte size, modification time,
content fingerprint, format, and optional producer/check scope. Available
empty, partial, missing, permission-denied, stale, malformed, cancelled,
timed-out, and worker-loss states remain distinct. Exact identity is
revalidated before every open. Raw log/report text, JSON, XML, and native
process output never cross into widgets as authority.

`c` opens cancellation confirmation for the exact active QA session. Managed
BitBake cancellation targets only the attached background job; layer-QA
cancellation targets only its matching native runner. Rejection restores the
running state with its reason. Cancellation never targets Testing, Security,
SDK, QEMU, Wic, Devtool, or unrelated metadata/report work.

Operation, import, and cancellation dialogs trap focus, show exact indexed
previews or normalized paths, and keep unavailable actions disabled with a
stable reason. All QA views and dialogs remain usable at 80×24. Wide mode uses
the persistent Navigator/Workspace/Inspector shell; medium and narrow modes
use the shared Inspector overlay and visible-pane switcher. Long paths,
findings, vectors, metadata, and limitations are bounded and wrapped. Every
theme and no-color mode preserves status meaning through text and terminal
attributes.

The full QA footer is:

```text
Tab view | ↑/↓ select | s scope | / search | f status | r run | I import | R refresh | Enter details | o report | e provider | l source | c cancel
```

At 90 columns and below it compacts to:

```text
Tab:view ↑↓ s:scope /:find f:status r:run I/R:data Enter o/e/l:open c:cancel
```

---

## 19. QEMU and Wic

### QEMU dialog

Integrate:

- `runqemu`
- `runqemu-extract-sdk`
- recognized networking helpers without exposing them as primary raw commands

Dialog fields:

- image
- machine
- kernel
- root filesystem
- networking
- memory
- display mode
- serial console
- extra arguments

The launch dialog opens only for an exact deployed root-filesystem or Wic
artifact that appears in the latest typed runqemu capability result. Image and
machine identity are read-only. Kernel and root-filesystem overrides must be
normalized absolute paths. Memory is entered in MiB and is bounded from 128
through 262,144. Networking, display, and serial console use typed choices;
`nographic` without a serial connection is invalid. Extra arguments are at
most 32 bounded runqemu keyword tokens: quoting, escaping, whitespace inside a
token, leading option markers, control characters, and shell metacharacters
are rejected.

`p` validates the editable draft and replaces it with a deterministic argument
preview. `Enter` in that preview starts the session. `Esc`
from either launch step returns without starting a process. Missing runqemu,
missing compatible images, failed capability inspection, stale artifact
identity, and validation failures are distinct visible states and never fall
back to a guessed command.

QEMU runs as a managed background job with an attached log/session view. Only
one managed runqemu session may be active. Its stable session identity retains
the exact launch request while the shared background-job state owns
queued/starting/running/cancelling/terminal timestamps, bounded typed stdout
and stderr, result, and error. Cancellation requires its own confirmation
dialog. Rejected cancellation restores the running state with a visible
reason; success, nonzero exit, cancellation, and process loss remain distinct
terminal results.

Images workspace QEMU shortcuts:

- `Q`: open the launch dialog for the exact selected compatible artifact
- `x`: request cancellation of the active managed session

The launch dialog begins on the read-only Machine row. `↑`/`↓` or `k`/`j`
moves across Machine, Image, Kernel, Root filesystem, Networking, Memory,
Display, Serial, and Extra arguments. Machine and Image cannot enter edit mode.
`Enter` begins editing a text row, ends the current text edit, or advances a
typed choice. `←`/`→` or `h`/`l` cycles typed choices backward/forward. `p`
validates and opens the exact preview. Text input is bounded to 4,096 bytes for
each optional path, 16 bytes for memory, and the model's aggregate 32-token
extra-argument bound. A validation failure remains visible in the draft and
does not close it.

`Esc` closes the launch draft even during text editing. In the preview,
`Enter` confirms and `Esc` closes without launch. In cancellation confirmation,
`Enter` confirms and `Esc` returns without changing the session. All three
dialogs trap focus and restore the previous pane when closed.

The Images Inspector presents runqemu before the longer selected-artifact
metadata: capability/executable, launch readiness or exact disabled reason,
then the latest session's exact request, lifecycle/timestamps, exit/result/
error, retained/drop/truncation counts, and stream-tagged output. Artifact
metadata follows it. This ordering keeps active session meaning visible in the
wide Inspector. Medium and narrow modes use the existing Inspector overlay/
pane switch without duplicating state.

Launch and preview overlays use the available supported terminal area and
remain usable at 80×24. Narrow launch titles retain `p preview` and `Esc close`;
the full row selection/edit/read-only markers remain visible. The global
below-80×24 resize message remains authoritative. The Images footer includes
`Q launch QEMU` and `x cancel QEMU` before scan/build/artifact actions.

### Wic dialog

Yoctui integrates the cooked-mode `wic create` workflow and the `wic write`
device workflow. It does not run Wic as root, guess raw-mode artifact
directories, or turn a free-form command string into the primary interface.
Missing tools, unavailable kickstarts, missing image artifacts, unsafe output
directories, unsupported preview syntax, and write-permission failures remain
distinct typed states.

`W` in Images opens Wic creation for the active machine and selected image
target. It is disabled until an exact image target, a canonical executable, and
at least one adapter-reported canned or configured kickstart are available.
The creation dialog contains:

- read-only machine identity
- typed image-target selection
- typed kickstart selection
- a normalized absolute output directory
- optional bmap generation
- a typed compression choice: none, gzip, bzip2, or xz

The initial selection prefers the active image and configured `WKS_FILE` only
when both identities occur in the latest typed inventories. `↑`/`↓` or `k`/`j`
moves between rows. `Enter` edits the output directory or advances a typed
choice; `←`/`→` or `h`/`l` cycles choices. `p` opens an exact shell-free
argument preview for cooked mode:

```text
wic create <kickstart> -e <image> -o <output-directory> [--bmap] [--compress-with <kind>]
```

The adapter resolves whether the selected kickstart is a canned name or a
canonical regular `.wks`/`.wks.in` file; widgets never derive a path from its
display name. The preview shows a bounded syntax-highlighted kickstart source,
each typed `part`/`partition` row, mount point, filesystem, source plugin, and
explicit size/alignment where present. Unknown or variable-derived values are
shown as `dynamic` or `unavailable`; Yoctui never fabricates a total image size.
At 80×24 the preview retains a syntax-highlighted two-line source excerpt with
the exact shown/total line count and compacts the indexed argument vector onto
wrapped rows. Medium and wide previews expand the source excerpt. The complete
bounded typed source and every typed partition remain in the Images Inspector;
the preview never reparses source text to derive partition data.
`Enter` in the command preview starts creation and `Esc` closes either step
without starting a process.

Creation is a managed background job. Its stable request retains the exact
machine, image, kickstart identity, output directory, options, and argument
preview. The shared job model owns lifecycle, bounded stream-tagged output,
cancellation, and terminal result. After success, the adapter scans only
new regular non-symlink files canonically beneath the requested output
directory and returns their exact path, kind, byte size, and modification time.
Empty and partial output inventories remain honest. The Images Inspector shows
the latest Wic request/job and generated outputs before general artifact
metadata. Capability/readiness precedes the latest operation; its status,
timestamps, result/error, retained bytes/entries, drop counts, warning/error
counts, and lowercase stream-tagged output are explicit. Generated inventory
states retain their generation and requested output root. Output rows show kind,
canonical path, byte size, modification time, and a visible selection marker.
The selected kickstart identity, canonical path or canned-name status, bounded
source, typed partitions, and adapter-reported limitations follow the generated
outputs without hiding the managed-QEMU section. `[`/`]` selects a generated
output and `O` opens it; lower-case `o`
continues to open the selected deployed artifact. `x` requests cancellation of
the active managed Wic operation when one exists, otherwise it retains its
managed-QEMU cancellation behavior.

`D` on an exact uncompressed `.wic` or `.direct` generated/deployed image opens
the protected removable-device flow. Device discovery is typed and bounded.
Only a whole block device reported removable, writable, large enough, and with
no mounted descendants is selectable. The current system/root backing device,
partitions, loop devices, device-mapper nodes, optical devices, and ambiguous
or stale identities are never eligible.

The device picker and confirmation show the exact image path and size plus the
device canonical path, major/minor identity, capacity, model, serial when
available, transport, removable/read-only state, and descendant mount summary.
The picker title identifies it as a protected write-device selection, uses a
visible `▶` marker in addition to semantic selection styling, and keeps partial
discovery limitations adjacent to the retained rows. Phrase entry and final
preview are separate overlays: the former states that typing alone cannot
write, while the latter labels the operation destructive and presents every
argument with its numeric index. The Images Inspector exposes write readiness,
the correlated protected-device inventory, exact write request identity,
host CPU/disk telemetry, retained and dropped output, truncation markers, and
terminal outcome across navigation.
Selection alone cannot write. The user must enter the exact phrase
`WRITE <canonical-device-path>` and then confirm the exact shell-free
`wic write <image> <device>` preview. Immediately before spawn, the adapter
rescans both identities and rechecks every safety invariant. Yoctui never
invokes `sudo`; insufficient device permissions produce a visible disabled or
failed state.

Device writing is a separate managed background job with bounded output and
distinct success, nonzero failure, cancellation, and process-loss results.
Cancellation requires a second confirmation warning that the device may be
incomplete. Every terminal result retains the exact image/device identity.
Fake block-device/process tests do not establish live removable-media safety;
that requires an explicit opt-in hardware smoke test.

All creation, preview, device picker, typed-phrase, command-preview, and
cancellation dialogs trap focus and render safely at 80×24. The Images footer
places `W create Wic` and `D write device` next to the existing lower-case `w`
shortcut, which continues to open an already-associated Wic path. Generated
output hints are `[/] select output` and `O open output`. At 90 columns and
below, the Images footer removes separators and abbreviates only the selection
and open labels so refresh, QEMU, Wic creation/cancellation, generated-output
selection/opening, deployed-artifact opening, and associated-Wic opening remain
visible on one line.

---

## 20. Maintenance workspace

Maintenance is a first-class Navigator destination. It must never reuse the
BBMASK screen or silently fall back to an unrelated workspace. It provides four
typed views in this fixed order:

1. `Sstate`
2. `Services`
3. `Release`
4. `Integrations`

`[` and `]` change view and preserve the selected row in every view. The wide
layout uses a capability/operation list in Workspace and exact configuration,
preview, result, and evidence in Inspector. Medium layout uses the standard
Inspector overlay. Narrow layout uses the standard pane switcher. Too-small
terminals render the standard safe message; every dialog below must render and
trap focus at 80×24.

The shared footer is
`[ ] view  r refresh  Enter inspect  x cancel  o open evidence  S signatures`.
View-specific enabled actions follow it: Sstate adds `c check` and `d cleanup`;
Services adds `e PR export` and `m PR import`; Release adds
`l locked cache`, `h compare`, and `a archive`; Integrations has detection and
inspection only. Disabled actions remain visible with a typed reason in
Inspector. `S` routes to the existing Signatures workspace. Security mapping,
recipe/kernel/layer QA, and recipe patch review remain owned by Security, QA,
and Recipes respectively; Maintenance links to those destinations instead of
duplicating their state or execution.

### 20.1 Capability and identity

Capability inspection records the canonical executable, detected interface,
supported operation family, exact configured roots, and an explicit
available/unavailable reason. The current initialized metadata is authoritative
for `SSTATE_DIR`, `TMPDIR`, `STAMPS_DIR`, `BUILDHISTORY_DIR`, `PRSERV_HOST`,
`BB_HASHSERVE`, `BB_HASHSERVE_UPSTREAM`, signature configuration, machine, and
distro. A missing metadata value or optional executable is unavailable, never
an empty successful capability.

All filesystem inputs are canonical absolute identities. Regular-file,
directory, symlink, containment, and read/write requirements are typed per
operation. Selection and preview retain the same exact identities. Immediately
before execution, the CLI re-inspects capability and revalidates every input;
changed identities reject the request visibly.

### 20.2 Sstate readiness and protected cleanup

Readiness uses the installed `oe-check-sstate` interface with one or more exact
target names. The default request runs its isolated-TMPDIR behavior and sets
`BB_SETSCENE_ENFORCE=1`; a separate explicit `same TMPDIR` choice is labelled
as dependent on prior build state. Its confirmation shows the shell-free
indexed argument vector, target list, selected mode, output/log paths, and
timeout. The result contains exact restored task names, totals, bounded output,
limitations, and terminal outcome. Readiness does not mutate the shared cache.

`c` opens the readiness form only when the exact readiness capability is
available. Targets begin empty and are entered as whitespace- or comma-separated
tokens; mode begins as `isolated TMPDIR`, output and log paths begin absent, and
timeout begins at 3600 seconds. `Tab`/`Shift+Tab` moves through Targets, Mode,
Output, Log, and Timeout. `Space` or `Left`/`Right` changes only Mode; normal
text edits the selected text field. `Enter` validates and requests an adapter
preview without running a command, while `Esc` closes without side effects.
Validation remains visible in the focus-trapped form.

Cleanup supports only an interface reported by capability inspection. Current
`sstate-cache-management.py` operations are typed as `duplicates`, `orphans`,
or `unreferenced by stamps`; legacy `.sh` support is a distinct detected
interface. The form shows canonical `SSTATE_DIR`, every canonical stamps
directory, worker count, and selected modes. Yoctui first performs a preview
without automatic confirmation and captures the exact candidate paths and
count. Execution requires the phrase
`DELETE <candidate-count> FROM <canonical-sstate-dir>` and then a separate
destructive confirmation showing the exact cache root, stamps roots, candidate
count, and indexed native vector. The final request uses the installed tool's
noninteractive confirmation flag only after Yoctui confirmation. It may delete
only the exact previewed regular files beneath the same cache root; a changed
candidate set, symlink, escape, identity, or capability rejects execution.
Arbitrary age-based deletion and free-form cleanup arguments are outside this
workflow.

`d` opens the cleanup form only when the exact cleanup capability and canonical
`SSTATE_DIR` are available. Cache and stamps roots are read-only; duplicates is
selected initially, orphans and unreferenced-by-stamps begin clear, and jobs
begins at one. `Tab`/`Shift+Tab` moves through the three modes and Jobs,
`Space` or `Left`/`Right` toggles a selected mode, and digits edit Jobs.
`Enter` validates and requests read-only candidate discovery; it cannot open
the deletion phrase or destructive confirmation until the adapter returns an
exact typed candidate preview. `Esc` closes without discovery or deletion.

Readiness and cleanup are independent cancellable background operations.
Cancellation of cleanup requires an additional warning that a partially
cleaned cache may remain.

### 20.3 PR and hash service diagnostics

Services renders typed configured, disabled, local, remote, reachable,
unreachable, partial, and unavailable states for the PR and hash services. It
shows exact configured endpoints and bounded observational process evidence
for `bitbake-prserv` and `bitbake-hashserv`. `bitbake-worker` is observational
build context only. Yoctui does not start, restart, stop, or reconfigure any of
these internal services and never treats process-name matching alone as proof
that a configured endpoint is healthy.

When the installed `bitbake-prserv-tool` supports it, PR export accepts one
canonical writable `.conf` or `.inc` destination and PR import accepts one
canonical readable regular `.conf` or `.inc` source. Both require a typed
preview and explicit confirmation because the native helper can stop a
memory-resident server and invalidate BitBake cache records; import is labelled
as changing PR data and receives destructive styling. The destination/source,
operation, configured endpoint, build identity, indexed native vector, and
known helper side effects are all visible. Undocumented helper commands are
not inferred or exposed.

`e` opens a PR export form and `m` opens a PR import form only when the exact
native helper, initialized build directory, and configured PR endpoint are
available. The shortcut fixes the operation; the build directory and endpoint
are read-only, while the file begins empty and accepts one canonical absolute
`.conf` or `.inc` path. Normal typing and `Backspace` edit that path. `Enter`
validates and requests an exact adapter preview without running the helper;
`Esc` closes without side effects. Both forms show the native server-stop and
cache-invalidation warning, import additionally states that it changes PR
data, and export states that it may replace the selected destination.

### 20.4 Release evidence

Locked-cache generation uses `gen-lockedsig-cache` with the exact ordered
inputs: locked-signature include file, input cache directory, output cache
directory, native LSB string, and optional filter file. Because matching
destination files may be replaced, the exact canonical output root and
replacement warning receive destructive styling and a separate explicit
confirmation. Completion returns a bounded inventory of created/replaced
evidence.

`l` opens the locked-cache form only when the exact generator capability and
authoritative native-LSB metadata are available. Locked-signature include,
input cache, output cache, and optional filter begin empty; native LSB is
read-only. `Tab`/`Shift+Tab` traverses those four editable paths in that order.
Normal typing and `Backspace` edit only the selected path. `Enter` validates
canonical absolute inputs and requests an adapter preview without running the
generator; `Esc` closes without side effects. The form always states that
matching files beneath the exact output cache may be replaced and that a
separate destructive confirmation remains required.

Build-history comparison uses `buildhistory-diff` with one exact canonical Git
repository and zero, one, or two validated revisions, plus typed report-version,
report-all, signature, signature-diff, exclude-path, and no-colour choices.
Its report is replaceable, bounded, and retains both resolved revisions.
`build-compare` is a separate optional capability and is disabled when absent;
it is never emulated by relabelling `buildhistory-diff`.

`h` opens the build-history comparison form only when the exact
`buildhistory-diff` capability and authoritative canonical `BUILDHISTORY_DIR`
repository are available. Repository is read-only. From revision, to revision,
and comma-separated exclude paths begin empty; report-version, report-all,
signatures, signature-diff, and no-colour begin clear. `Tab`/`Shift+Tab`
traverses those fields in that order, with exclude paths between signature-diff
and no-colour. Normal typing and `Backspace` edit only text fields, while
`Space` or `Left`/`Right` toggles only the selected choice. `Enter` validates
and requests an exact adapter preview without running a comparison; `Esc`
closes without side effects. The form labels bounded session output and states
that `build-compare` is a separate unsupported interface.

Git archival uses `oe-git-archive` with exact data and repository directories,
typed create/bare/tag choices, branch/tag/message templates, exclusions, and
notes. Push is never implicit. Local archive creation has an exact preview;
repository creation, tag replacement risk, or overwriting tracked output is
called out in confirmation. A requested remote push is a second network side
effect requiring a separate explicit confirmation after the local result.

`a` opens the Git archive form only when the exact `oe-git-archive` capability
is available. Data and Git directories begin empty. Create and create-tag begin
selected, bare begins clear; branch, tag, commit-subject, and tag-subject begin
as `release/{machine}`, `release/{tag_number}`, `Release {commit}`, and
`Release tag {tag_number}`. Commit/tag bodies, comma-separated exclusions,
comma-separated `reference=/absolute/file` notes, and push remote begin empty.
`Tab`/`Shift+Tab` traverses those fields in the displayed order. Normal typing
and `Backspace` edit text; `Space` or `Left`/`Right` toggles only create, bare,
and create-tag. `Enter` validates and requests an exact adapter preview without
creating, tagging, or pushing; `Esc` closes without side effects. A non-empty
push remote records intent only: local archival is confirmed and completed
first, then push requires the separate network confirmation. The form always
shows repository-creation, tag-replacement, and tracked-output overwrite risk.

### 20.5 Optional integrations

Integrations detects and reports, without launching:

- `create-pull-request` and `send-pull-request`, including canonical helper and
  Git-worktree identity
- `send-error-report`, including the helper and candidate report identity
- a canonical repo manifest or an explicit unavailable reason
- Toaster executable/configuration and observational running state

This milestone is detection-only for pull-request email, error-report upload,
repo-manifest mutation, and Toaster lifecycle. No key in this view sends mail,
uploads data, changes a manifest, or starts/stops Toaster. Future execution
must add a typed review of recipients/server/payload or manifest changes and a
separate network-side-effect confirmation.

### 20.6 Execution, evidence, and validation

Capability and service inspection use replaceable correlated workers.
Maintenance owns at most one foreground operation runner; managed BitBake
operations continue to use the shared build coordinator. Every session has a
stable operation ID, exact context, queued/running/cancelling/terminal state,
bounded stdout/stderr with dropped-byte counters, optional typed progress,
start/end time, timeout, exit status, evidence identities, and explicit
success, failure, cancelled, timed-out, or runner-lost outcome. Navigation does
not discard sessions. Stale events cannot replace current capability, preview,
or evidence.

`x` opens a confirmation only for the exact cancellable active operation.
Evidence opening requires a canonical regular file from the current successful
operation and repeats containment/identity validation immediately before the
editor or viewer transition. Reports are replaced atomically only after
successful completion; failures retain prior valid evidence and the failed
attempt's bounded output.

Fake process/filesystem tests prove typed vectors, validation, lifecycle, and
rendering only. They do not establish live cache safety, service health, PR
database compatibility, signature-cache compatibility, archive correctness, or
network interoperability. Those claims require explicit opt-in validation in
an initialized Yocto environment, and destructive/network validation requires
dedicated disposable resources.

---

## 21. Dialog system

All dialogs use a common framework.

Dialog types:

- build
- confirmation
- text input
- selection list
- multi-field form
- progress
- command result
- destructive action
- external tool launch
- error

Common rules:

- title
- concise description
- clear focus
- keyboard navigation
- `Enter` activates primary action
- `Esc` cancels
- destructive action button is visually distinct
- validation is inline
- unavailable submit action explains why
- long-running dialog actions become background jobs
- dialogs must not block backend event consumption
- only the active typed dialog receives input or renders
- invalid actions for the active dialog leave it unchanged
- asynchronous result dialogs retain FIFO order behind an active user dialog

---

## 22. Notifications

Notifications appear above the footer or in a temporary overlay.

Types:

- info
- success
- warning
- error
- progress

Notifications support:

- timeout
- persistent state for important failures
- action shortcut
- grouping repeated messages
- build-session association
- screen-reader/plain-text fallback

Do not flood the UI with one notification per BitBake log line.

---

## 23. Command palette

`Ctrl+P` opens a searchable command palette.

Examples:

- Build target
- Open Layers
- Open current task log
- Run menuconfig
- Start QEMU
- Generate Wic image
- Run sstate readiness check
- Show dependency path
- Open settings
- Switch theme
- Toggle reduced motion

Commands are filtered by context and availability.

Unavailable commands remain discoverable but explain their requirements.

The palette uses one typed catalog with stable ordering. Each result shows:

- action label
- concise description
- existing shortcut, or `none`
- selected state
- `unavailable` state and its exact requirement

Typing filters case-insensitively across labels, descriptions, and shortcuts.
`Backspace` edits the query, `Up`/`Down` moves through filtered results,
`Enter` activates an available result, and `Esc` closes the palette. Empty
results show an explicit message. Activating an unavailable command or an
empty result changes no application state.

Palette input is routed before dialog and workspace input and remains
focus-trapped. Opening records the exact active pane. Closing without a
command restores that pane; navigation commands move focus to their selected
workspace, while commands that open dialogs preserve the original pane return
target through the dialog workflow.

---

## 24. Footer and keyboard shortcuts

The footer is always visible in normal layouts.

It shows context-sensitive shortcuts, not a fixed oversized list.

Global example:

```text
? Help  F5 Build  Ctrl+P Commands  / Search  Tab Focus  e Errors  l Logs  q Quit
```

Layers example:

```text
Enter Open/Toggle  ← Collapse  → Expand  e Editor  m Metadata  d Dependencies  / Search
```

Tasks example:

```text
↑/↓ Select  f State  F Field  / Edit Filter  d Duration  c Cancel  Tab Focus
```

Dialog example:

```text
Tab Next  Shift+Tab Previous  Space Toggle  Enter Confirm  Esc Cancel
```

Rules:

- shortcuts must reflect the active focus and workspace
- disabled shortcuts are dimmed
- no hidden critical action
- help screen lists all global and context shortcuts
- configurable keymaps may be added later, but defaults remain stable

---

## 25. Themes and preferences

Configuration file:

```text
$XDG_CONFIG_HOME/yoctui/config.toml
```

Required built-in themes:

- `dark-pro`
- `white-classic`
- `matrix-green`
- `vscode-dark`
- `vscode-light`
- `accessible-dark`
- `soft-light`
- `high-contrast`

These palettes match Packrat's built-in theme set exactly. `--no-color`
remains a separate accessibility override rather than a selectable palette.

Example:

```toml
[ui]
theme = "dark-pro"
animation_speed = "fast"
reduced_motion = false
show_icons = true
unicode = true
compact_header = false
footer_shortcuts = true
mouse = true
refresh_hz = 30

[ui.panes]
navigator_width = 22
inspector_width_percent = 38
remember_sizes = true

[logs]
wrap = true
follow = true
max_entries = 100000
max_bytes = 67108864
```

### Matrix green theme

Matrix green must remain usable, not decorative noise.

Suggested semantics:

- background: black
- primary text: green
- focused border: bright green
- inactive text: dark green/gray
- success: bright green
- warning: yellow-green or yellow
- error: high-contrast red
- selected row: reverse or bright-green background with black text
- progress animation: multiple green intensities

Themes must preserve semantic distinctions. Errors cannot become indistinguishable from success.

### Semantic roles

Rendering uses one complete palette per built-in theme. Widgets select a role,
not a terminal color:

- foreground and background
- inactive and focused borders
- selected foreground and background
- disabled or subdued text
- informational accent
- success, warning, and error
- determinate and indeterminate progress
- general text accent
- source keyword, name, operator, value, and comment

The persistent shell, workspaces, Inspector, Footer, dialogs, notifications,
tables, gauges, logs, build status, and source preview use these roles. A
theme must provide every role. Adding a role requires updating all built-in
themes and deterministic TestBackend coverage.

`monochrome` and `--no-color` use terminal attributes instead of color:

- focused elements are bold
- selections use reverse video
- disabled text is dim
- warnings are bold
- errors are bold and underlined

These modes must not depend on the terminal's default foreground/background
pair to distinguish focus, selection, severity, or progress.

### Theme switching

Theme can be changed through:

- Settings workspace
- command palette
- CLI/configuration

In the Settings workspace, activating the Theme row opens a focus-trapped theme
submenu. Up/Down selects a named theme and applies it immediately; Enter keeps
the selection and Esc closes the submenu. Theme selection is never a blind
toggle or an implicit cycle.

Theme changes apply immediately and persist.

### Preferences

The Settings workspace is a typed row editor. `Up`/`Down` (or `j`/`k`) selects
a row; `Left`/`Right` or `Enter` changes its value. The supported rows are:

- theme
- animation speed
- reduced motion
- color enablement
- log wrapping
- log following

Changes preview immediately and are atomically saved to `session.toml`.
`config.toml` is a user-authored default and is never rewritten by the TUI.
Session values override configuration defaults for these interactive rows;
hard CLI overrides such as `--no-color` remain authoritative. A failed save
keeps the previewed value, marks Settings as unsaved, and shows a notice.
Pressing `r` retries the atomic save without changing the previewed value.

Persist:

- theme
- animation speed
- reduced motion
- selected workspace
- pane sizes
- wrap/follow modes
- filters
- recent targets
- recent build directories
- editor
- backend
- mouse preference
- compact layout preference

Do not persist live BitBake state as authoritative state.

---

### Build environment workspace

`Build environment` is a dedicated Navigator destination immediately above
`Settings`; it is not a general Settings row. Yoctui launches without
positional targets or a `--build-dir` argument with Navigator focus. If it does
not have a previously verified build environment, it selects this destination
and keeps the Workspace focus available for navigation. Until the connection check passes,
build and metadata actions remain visible but disabled with the reason
`Configure and verify a BitBake environment first`.

The Build environment workspace is a typed setup form with these rows:

Activating any editable setup row opens a bounded, focus-trapped popup editor.
The Build environment profile popup is a TOML document with a vi-like Normal
and Insert mode: `i` enters Insert, Esc returns to Normal, Enter applies the
validated document, and `q` closes without applying. Paste and literal path
characters are accepted as document input.

### Editable-popup convention

Every editable workflow uses a bounded, focus-trapped popup rather than an
inline text field. Structured settings are presented as TOML documents with
their typed field names. Popup editors use vi-like Normal and Insert modes:
`i` enters Insert, Esc returns to Normal, Enter validates/applies, and `q`
closes without applying. Existing destructive confirmation dialogs remain a
separate explicit step after validation; a popup editor never bypasses them.

- **Use existing source**: an absolute Poky/Yocto source path, an absolute
  build-directory path, and the detected environment script
  (`oe-init-build-env` or a build wrapper).
- **Clone Poky**: repository URL, absolute destination, optional revision, and
  absolute build-directory path. Clone never starts before a review screen
  shows the exact non-shell `git clone` and checkout vectors and the user
  confirms them.
- **Initialize environment**: runs only the selected, validated environment
  script for the selected build directory. Yoctui captures the resulting child
  environment for its managed BitBake processes; it never mutates the TUI
  process environment.
- **Open setup shell**: opens an inherited embedded shell at the selected
  source/build context when initialization needs an interactive answer. The
  shell owns all input until it exits; after exit Yoctui rechecks the selected
  environment rather than assuming success from terminal text.
- **Verify connection**: checks the selected build directory and starts the
  managed BitBake connection. Success requires a typed workspace response;
  failure shows its bounded diagnostic and keeps build controls disabled.
- **Available images**: appears only after verification succeeds and lists the
  typed image recipes returned by BitBake. Selecting an image enables the build
  action; no image is guessed from a filename or shell transcript.

Source, build, and clone destinations must be absolute canonical directories
when used. Existing paths are inspected before initialization; a missing,
unsafe, or mismatched script is explained without execution. Clone may create
only its reviewed destination, never overwrites a nonempty directory, and is
cancellable before initialization. Initialization, shell exit, clone, and
verification retain distinct pending, success, cancelled, and failure states.

The selected source/build profile and recent verified profiles may persist in
the session as suggestions; recent paths alone never auto-connect or override
unconfigured startup. Captured environment values, credentials, live server state, and
terminal transcripts do not persist. `--build-dir` and `--backend` remain
supported diagnostic overrides; normal interactive startup does not require
users to understand backend names such as `bridge`. A verified profile can be
replaced from this workspace; replacing it clears the active connection and
returns build controls to the disabled state until the new source is initialized
and verified.

### Theme rendering contract

Every theme selection must change the semantic palette used by the complete
shell, including Navigator focus, workspace selection, dialog borders,
notifications, status/severity, and the Build environment workspace. Theme
changes must be visible immediately in both wide and narrow layouts and must
not leave stale colors from the previous theme. No-color mode remains an
attribute-only override.

## 26. Responsive layouts

### Wide terminal

At widths of 130 columns and above, use navigator + workspace + inspector.

### Medium terminal

At widths from 100 through 129 columns, keep navigator and workspace. Focusing
the Inspector with Tab or Shift+Tab replaces the workspace region with an
Inspector overlay; Shift+Tab or Esc returns to the workspace and Tab continues
the focus cycle.

### Narrow terminal

At widths from 80 through 99 columns, use one pane at a time with a visible
Navigator / Workspace / Inspector switcher. Tab and Shift+Tab cycle the active
pane. The same focus selection is retained across resize transitions.

### Too small

Widths below 80 columns or heights below 24 rows show only the resize message.

Show:

```text
Yoctui needs at least 80x24.
Current terminal: 62x18.
Resize the terminal or press Q to quit.
```

No layout may panic due to terminal dimensions.

---

## 27. Mouse support

Mouse support is optional and configurable.

When enabled:

- click selects rows/tree nodes
- wheel scrolls focused pane
- click tabs changes inspector mode
- pane borders may be draggable in a future version

Every action must remain fully usable by keyboard.

---

## 28. Background jobs

Builds, QEMU, Wic creation, tests, SDK creation, Devtool actions, and maintenance commands are background jobs.

A background job has:

- identifier
- type
- title
- status
- start/end time
- progress when available
- output
- warnings/errors
- cancellation support
- related workspace item

The user can browse layers and files while a build or other job continues.

---

## 29. Safety rules

- never silently edit metadata or configuration
- never run destructive clean/cache/device operations without confirmation
- show the exact command before advanced operations
- show affected paths
- distinguish UI quit from build cancellation
- preserve errors even when ordinary logs are dropped
- restore the terminal on all supported exits
- never display secrets from the environment by default
- redact likely credentials in diagnostics

---

## 30. Implementation contract for the agent

The implementation agent must:

1. Read this file before changing UI behavior.
2. Treat it as authoritative.
3. Implement the persistent shell before adding more disconnected screens.
4. Use the shared focus model.
5. Use the shared dialog system.
6. Use the shared footer shortcut system.
7. Consume typed backend events rather than parsing output in widgets.
8. Preserve build activity while navigating other workspaces.
9. Add tests for every interaction change.
10. Update this file in the same commit when intentionally changing the UI contract.

The agent must not continue implementing unrelated feature checkboxes when a user request changes this specification.

When the user provides a new UI requirement:

1. pause unrelated implementation
2. update this document
3. update tests
4. implement the requirement
5. verify the behavior
6. commit the coherent change
7. then resume the implementation-status checklist
