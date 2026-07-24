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
- package, image, SDK, testing, QEMU, Wic, sstate, CVE, SPDX, and maintenance workflows

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
│ Devtool          │                                     │                                  │
│ QEMU / Wic       │                                     │                                  │
│ Maintenance      │                                     │                                  │
├──────────────────┴─────────────────────────────────────┴──────────────────────────────────┤
│ F1 Help  F5 Build  / Search  Tab Focus  Ctrl+P Commands  E Errors  L Logs  Q Quit        │
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
- open directory or file in configured external editor/file manager
- refresh selected subtree
- detect modified, untracked, and generated files where Git information is available

Keyboard:

- `Right` / `l`: expand
- `Left` / `h`: collapse or move to parent
- `Enter`: open file or toggle directory
- `e`: open in editor
- `r`: refresh
- `.`: toggle hidden files
- `/`: search
- `g`: Git details
- `m`: metadata view
- `d`: dependencies view

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

The first version does not implement a full embedded code editor.

Pressing `e` launches `$EDITOR` or the configured editor. After it exits, Yoctui refreshes the file, Git status, and affected metadata.

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

`F5` or `b` opens a build dialog.

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
- a reduced-motion preference disables or slows animation

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

Destructive actions require explicit confirmation.

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

- center: navigable dependency list/tree
- inspector: selected node details and path explanation

Graph rendering must degrade gracefully in terminals. A tree/path view is mandatory; a visual graph is optional.

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

Editing configuration requires a dedicated preview-and-confirm dialog.

No silent edits.

---

## 18. Packages, images, SDK, and test results

### Packages

Integrate `oe-pkgdata-util`.

Show:

- package name
- recipe
- files
- runtime dependencies
- reverse dependencies
- size
- image membership

### Images

Show:

- image artifacts
- file sizes
- timestamps
- manifests
- licenses
- SPDX/SBOM artifacts
- Wic images
- deploy directory
- checksums

### SDK

Integrate:

- `do_populate_sdk`
- `do_populate_sdk_ext`
- `do_testsdk`
- `do_testsdkext`
- `oe-publish-sdk`
- `oe-run-native`
- `oe-find-native-sysroot`

### Testing

Integrate:

- `resulttool`
- `oe-selftest`
- `bitbake-selftest`
- `do_testimage`
- `do_testsdk`
- `do_testsdkext`
- `ptest`

Show regressions, newly failing tests, newly passing tests, logs, metadata, and JUnit export.

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

QEMU runs as a managed background job with an attached log/session view.

### Wic dialog

Integrate:

- `wic create`
- kickstart selection
- image selection
- output directory
- size and partition preview
- confirmation for writing removable devices

Direct device writes require strong confirmation and a clear device summary.

---

## 20. Maintenance workspace

Advanced and potentially destructive operations live here.

Integrations include:

- `oe-check-sstate`
- `sstate-cache-management.sh`
- `buildhistory-diff`
- `build-compare`
- `bitbake-diffsigs`
- `bitbake-dumpsig`
- `gen-lockedsig-cache`
- `bitbake-prserv-tool`
- `cve-check-map-pkgs`
- `yocto-check-layer`
- `patchreview`
- `send-error-report`
- `create-pull-request`
- `send-pull-request`
- `oe-git-archive`

Internal services such as `bitbake-worker`, `bitbake-prserv`, and `bitbake-hashserv` are observed and diagnosed, not normally launched directly.

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

---

## 24. Footer and keyboard shortcuts

The footer is always visible in normal layouts.

It shows context-sensitive shortcuts, not a fixed oversized list.

Global example:

```text
F1 Help  F5 Build  Ctrl+P Commands  / Search  Tab Focus  E Errors  L Logs  Q Quit
```

Layers example:

```text
Enter Open  ← Collapse  → Expand  E Editor  M Metadata  D Dependencies  / Search
```

Tasks example:

```text
Enter Inspect  F Follow  A Active  R Recipe Filter  C Cancel  E Errors  L Logs
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

- `dark`
- `light`
- `matrix-green`
- `high-contrast`
- `monochrome`

Optional future themes:

- `nord`
- `gruvbox`
- `solarized-dark`
- `solarized-light`

Example:

```toml
[ui]
theme = "matrix-green"
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
