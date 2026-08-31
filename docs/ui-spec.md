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

The normal application layout is a dense, IDE-like operations workbench:

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ yoctui · Project: core-image-minimal · Machine: qemux86-64 · Distro: poky  Daemon: Connected · BitBake: Running │
├──────────────────┬─────────────────────────────────────┬──────────────────────────────────┤
│ Navigator        │ Workspace                           │ Inspector                        │
│                  │                                     │                                  │
│ ▾ OVERVIEW       │ Context-specific list/tree/table    │ Preview/details/live output      │
│   Dashboard      │ with compact titled subpanels       │ and context actions              │
│ ▾ CONTENT        │                                     │                                  │
│   Layers         │                                     │                                  │
│   Recipes        │                                     │                                  │
│   Packages       │                                     │                                  │
│   Images         │                                     │                                  │
│ ▾ BUILD          │                                     │                                  │
│   Tasks          │                                     │                                  │
│   Logs           │                                     │                                  │
│   Errors         │                                     │                                  │
│ ▾ VALIDATE       │                                     │                                  │
│   Testing        │                                     │                                  │
│   Security / QA  │                                     │                                  │
├──────────────────┴─────────────────────────────────────┴──────────────────────────────────┤
│ ↑/↓ Select  f State  / Filter  c Cancel  F1 Help  F10 Menu  q Quit          19:28:27   │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

The shell contains five persistent regions:

1. Header
2. Navigator
3. Workspace
4. Inspector
5. Shortcut/status footer

Dialogs and notifications are drawn above this shell.

### Workbench visual language

The reference visual direction is a compact professional terminal IDE. The
default `dark-pro` rendering must use near-black panel surfaces, thin subdued
borders, a saturated blue full-row selection, lime success/progress, amber
Navigator group and folder accents, cyan links/information, and red failure.
Other themes map the same semantic roles through their own palettes.

The shell is deliberately dense: panel titles consume one border row, tables
use one row per record, and decorative blank space is avoided. Adjacent panels
share a visually continuous grid. Focus remains visible through the focused
border and selection treatment; color is never the only status signal.

Exactly one bordered region carries the focused-border role at a time. A
multi-section Inspector assigns that role to its primary titled section; its
secondary facts, recent output, actions, and system-status sections retain
inactive borders while remaining within the same focused pane. Dialog focus
removes focus styling from the shell and assigns it only to the modal border.

### Literal reference acceptance

The approved terminal reference is an acceptance target, not merely visual
inspiration. At the canonical `160x48` terminal size the default `dark-pro`
Tasks workspace uses this exact application-controlled cell geometry:

| Region | Rectangle |
| --- | --- |
| Header | `x=0, y=0, width=160, height=2` |
| Navigator | `x=0, y=2, width=26, height=44` |
| Tasks table | `x=26, y=2, width=89, height=17` |
| Log Viewer | `x=26, y=19, width=89, height=18` |
| Job History | `x=26, y=37, width=89, height=9` |
| Task Inspector | `x=115, y=2, width=45, height=16` |
| Recent Log | `x=115, y=18, width=45, height=15` |
| Actions | `x=115, y=33, width=45, height=7` |
| System Status | `x=115, y=40, width=45, height=6` |
| Command rail | `x=0, y=46, width=160, height=2` |

The machine acceptance artifact serializes every Ratatui cell's symbol,
foreground, background, underline color, and modifiers. The reference fixture
uses a fixed clock and typed model values; dynamic fields are not masked. A
golden update is an intentional UI change and requires a reviewed cell diff
plus a matching update to this specification. Normal verification never
automatically accepts new goldens.

The canonical scene shows `core-image-minimal`, `qemux86-64`, `poky`, a
connected daemon, and an active `bash:do_compile` task at 72 percent. Because
that task is active, the header says `BitBake: Running`; the contradictory
`Idle` label in the illustrative raster is deliberately corrected. Test-only
fixture values never enter production state. Live rendering uses the same
geometry with authoritative BitBake values.

The code-owned acceptance level is the terminal cell buffer. A raster capture
is supporting human evidence only because terminal font, glyph rasterization,
DPI, and compositor behavior are outside Yoctui's control.

Semantic TestBackend snapshots complement the literal golden. They use the
same fixed clock and typed fixture authority, but compare stable region titles,
state text, selected-row styling, and dialog controls rather than incidental
blank cells. The required semantic catalog covers Dashboard, Tasks, Logs, Job
History, Recipes, Layers, Images, Settings, Build Environment, a real typed
terminal-session pane, and standard, confirmation, destructive, result, and
editor dialogs. Each catalog entry names its reviewed anchors in code; adding
or removing an anchor is therefore an intentional test review, while spacing
inside an unrelated pane is not a snapshot update.

Four target-design goldens use `160x50` and the fixed `19:28:27` clock: an
idle Dashboard with current daemon authority, the active Tasks build at 72%,
the same Tasks cockpit with `bash:do_compile` selected and failed, and a Tasks
cockpit while the daemon replica is synchronizing. These serialize every cell
symbol and style. `YOCTUI_UPDATE_TARGET_GOLDENS=1` is the only update switch;
the repository script runs it and prints the four fixture diffs for review.
Normal verification only compares and reports the first changed coordinate.

No renderer may copy illustrative values from a design reference. Every value
comes from typed model state; missing values read `unavailable`, `unknown`, or
`--` according to the field contract.

### Next-generation layout contract

This section is the normative M19 layout contract. It refines the earlier
literal scene without invalidating the reviewed `160x48` acceptance artifact:
that artifact remains a regression fixture until a later target-design golden
task reviews and replaces it. New canonical scenes use `200x60` and `160x50`
so the telemetry/context tier can be exercised without removing the task,
log, or history tiers.

The persistent region hierarchy is:

```text
Shell
├── Header
├── Body
│   ├── Navigator
│   ├── Workspace
│   │   ├── Main workspace
│   │   ├── Secondary workspace / log / details
│   │   └── Context tier
│   │       ├── History or workspace context
│   │       └── Telemetry strip, when supported and space permits
│   └── Inspector
│       ├── Title and primary facts
│       ├── Secondary facts / related paths
│       ├── Recent bounded output
│       ├── Contextual actions
│       └── System or compatibility status
└── Footer
    ├── Contextual shortcut rail
    └── Transient status and fixed-width clock
```

Header and footer are each exactly two terminal rows in every supported
layout. The body owns every remaining row. Panels use one-cell borders and
one-cell title rows; adjacent rectangles must not overlap. A renderer must
return before attempting to split an empty rectangle.

#### Dimensions and breakpoints

The supported minimum is `80x24`. Both dimensions are mandatory. A terminal
below either minimum renders only the resize message and the quit route; it
does not attempt a partial shell.

At `130` columns and wider, all three body panes are visible. Column sizing is
deterministic:

- Navigator: approximately `16.25%`, clamped to `22..30` columns
- Inspector: approximately `28.125%`, with a `32`-column minimum
- Workspace: all remaining columns, never less than `76` at the wide boundary

This produces `26 / 89 / 45` at 160 columns. At `200x60`, the preferred
allocation is `30 / 116 / 54`. At `130x40`, it is `22 / 76 / 32`. The
Workspace is the first recipient of extra width after the Navigator reaches
30 columns; the Inspector may then grow without displacing Workspace below
its minimum.

At `100..129` columns, Navigator is 22 columns and Workspace receives the
remainder. Inspector remains a first-class focus target but is collapsed from
the grid. Focusing it replaces the Workspace rectangle with a full-height
Inspector overlay; `Esc` returns to Workspace and Tab/Shift+Tab preserve the
global focus order.

At `80..99` columns, exactly one body pane is visible beneath a one-row pane
switcher. Navigator, Workspace, and Inspector retain independent selection and
scroll state while hidden. Tab and Shift+Tab change the visible pane. Dialogs
replace none of those states and remain bounded inside the full terminal.

Height degradation is independent of width:

- body height `46+` (for example `160x50`): Main, Secondary, History/Context,
  and supported Telemetry tiers may all render
- body height `36..45`: omit the Telemetry tier first and give its rows to Main
  and Secondary; the reviewed `160x48` Tasks geometry remains valid
- body height `27..35`: retain Main, Secondary, and a compact Context summary
- body height `18..26`: retain Main and Secondary; history moves to Inspector
- body height `1..17`: render only Main with its title and bounded rows

The canonical responsive verification matrix is `200x60`, `160x50`,
`130x40`, `100x30`, `80x24`, and `79x23`. The first three exercise the wide
three-pane shell, `100x30` exercises both the normal medium Workspace and the
focused Inspector replacement, `80x24` exercises each single-pane switcher
state, and `79x23` exercises the exclusive resize screen. Dashboard, Tasks,
Logs, Recipes, and Layers must retain meaningful selected-state text at every
supported size; a typed Build Options dialog must retain its title, primary
field, confirm action, and close hint. Canonical buffers must contain neither
replacement characters nor clipped dialog controls, and resizing across the
matrix must not mutate pane focus or workspace selection identity.

Pane priority is therefore: persistent Header/Footer, focused modal, active
Workspace Main, Navigator or narrow pane switcher, Inspector primary facts,
Secondary Workspace, contextual actions, History, System Status, Telemetry,
and decorative detail. Removing a low-priority region must never remove its
keyboard route or authoritative detail; the detail moves to Inspector/help or
is reached by the existing workspace route.

#### Workspace tier allocation

Each workspace declares which of Main, Secondary, and Context it supports.
Unsupported tiers are omitted and their rows return to Main. Tasks uses Main
for the task table and overall build summary, Secondary for the selected live
log, and Context for job history plus telemetry. Logs uses Main for retained
entries and a compact status/search section; its selected record details live
in Inspector rather than a duplicate fake tail. Other workspaces keep their
existing typed tables, trees, previews, and detail panels and may use a single
Main region.

At `160x50`, the target Tasks Workspace body uses 14 rows for Main, 14 for
Secondary, 10 for History, and 8 for Telemetry when at least one telemetry
group is supported. If telemetry is wholly unavailable, its 8 rows return to
Main and Secondary in equal proportions. At `200x60`, additional rows first
extend Main and Secondary, then History; the telemetry strip remains bounded
to 8 rows.

#### Inspector collapse and priority

Wide Inspector sections are ordered: primary facts, secondary facts/paths,
recent bounded output, contextual actions, then system/compatibility status.
The selected entity type is always named in the title. At reduced height:

1. recent output shrinks to a compact tail and then becomes a Logs route
2. secondary facts collapse behind the same Inspector scroll region
3. system status becomes a compact health line
4. primary facts and at least one enabled action remain visible

Disabled actions may remain visible when their exact typed reason is useful.
They use the disabled semantic role and cannot look selected or enabled.
Medium uses the overlay described above. Narrow uses the pane switcher and a
single independently scrollable Inspector. No breakpoint creates a second
Inspector authority or selection.

#### System Status presentation

System Status is a bounded four-line projection, not a second daemon model.
At 40 or more content columns it orders: daemon connection/version/uptime;
BitBake lifecycle/version/active jobs; PTY sessions/connected clients/current
compatibility generation and counts; then build-filesystem capacity and exact
workspace path. Below 40 columns the same facts use shorter labels and stable
priority. Every line is bounded with a visible ellipsis rather than wrapping
into the next section.

Daemon uptime and active-job telemetry, BitBake lifecycle, PTY inventory, and
connected-client count render only while the client replica is `Current`.
Disconnected, Synchronizing, or Stale replicas name their exact connection
state and render those retained facts as `unavailable`; stale numeric values
must never look current. Compatibility authority likewise requires a Current
replica and otherwise says Unavailable. Workspace and build-filesystem facts
remain independently valid when supplied by local typed workspace and host
telemetry state. The current protocol supplies no daemon version or PID, so
System Status explicitly says `version unavailable` and never invents either.
The misleading queue-depth alias and assumed-page-size resident-memory
diagnostic remain non-renderable.

Every status line begins with a non-color marker and uses the matching semantic
role: `✓` healthy/current, `…` synchronizing or lifecycle transition, `!`
warning/degraded/stale, `✕` failed/disconnected/error, and `–` unavailable or
inactive. The persistent header uses the same markers for daemon and BitBake;
BitBake is explicitly unavailable whenever daemon authority is not Current,
even if a retained lifecycle exists. Reduced motion never animates markers.

Build-filesystem health uses the same whole used percentage as its gauge:
below 70% is healthy, 70–89% is warning, and 90% or greater is error. Invalid
or missing capacity is warning/unavailable rather than zero. Compatibility
Full is healthy, Degraded or Diagnostic is warning, and absent/non-current
authority is warning/unavailable. Any retained log eviction is log pressure;
an evicted error elevates it to error. Unknown workspace identity is an
explicit warning. The fourth bounded System Status line prioritizes unknown
workspace, then log pressure, while retaining valid filesystem/workspace
context as width permits.

#### Footer behavior

The footer is a bounded current-context projection, not a decorative fixed
F-key list. It prefers, in order: active dialog or confirmation controls,
active search/editor controls, up to six current workspace actions, one
compact focus destination, optional non-current workspace routes, and global
help/menu/quit. Whole hints are added only when they fit; a lower-priority hint
is omitted rather than clipped. Complex narrow workspaces may use documented
compound key tokens such as `s/E SDK` to retain more real controls. Help lists
the complete shared function-key catalog and every valid binding omitted for
width.

Transient status shares the footer rather than covering the Workspace. Its
priority is exact error, pending confirmation, notification or operation
result, daemon/BitBake synchronization, then local background/build activity.
An arbitrary notification is informational; it becomes error or warning only
when an exact retained log entry or the typed build-completion transition
provides that severity. Stale daemon state says `Daemon state stale`; it is not
misrepresented as reconnecting. Disconnected state stays in Header/System
Status instead of creating permanent transient noise.

The status slot is inserted immediately before the clock. Its desired width is
44 cells at 180+, 36 at 130–179, 28 at 100–129, and 26 below 100 columns, but
it shrinks or disappears before consuming the 36-cell wide/medium or 32-cell
narrow critical-shortcut reservation. Text whitespace is normalized onto one
line and bounded with a visible ellipsis. Semantic marker-plus-text forms are
`✕` error, `!` confirmation/warning, `✓` success, `i` information, `…`
synchronizing, and `▶` activity, so no-color and reduced-motion retain the
same meaning. The fixed-width clock remains last at 100+ columns and is hidden
below 100 columns.

#### Search behavior

Every searchable workspace keeps its existing typed query, result selector,
and reducer actions; there is no generic shell-backed or cross-workspace
search mutation. The shared presentation is one bounded line with this
semantic order:

```text
/ Search [EDITING|FILTERED|IDLE] · Query: <text|empty> · Results: <current>/<total> · <navigation> · Ctrl+U clear · <finish/edit hint>
```

`EDITING` plus a trailing `▏` text cursor means keyboard text is trapped by
that search input. `FILTERED` means a non-empty query remains applied after
input finishes. `IDLE` means the query is empty. `Results` counts the actual
domain-filtered rows; it is `0/0` for no matches and otherwise follows the
selected result identity/index. Logs advertise `n/N next/previous`; other
lists and the command palette advertise `↑/↓ results`. `Enter done` and `Esc
done` finish editing while retaining the filter, `/ edit` resumes it, and
`Ctrl+U clear` clears the active domain query in one typed action. The command
palette remains focus trapped and uses `Esc close` instead of finishing into a
workspace.

Wide layouts show the complete line. Medium layouts may omit navigation words
after retaining mode, bounded query, and numeric results. Narrow layouts may
reduce this to `/ [EDITING|FILTERED|IDLE] <query> <current>/<total>` plus the
cursor, but never hide editing focus or numeric result state. Long queries are
ellipsized by terminal cells, controls are never rendered as query content,
and high-contrast/no-color/reduced-motion modes preserve the same bracketed
state and text cursor.

#### Telemetry strip behavior

Telemetry is presentation of sampled typed state, never an illustration.
Each cell includes a text value even when it also uses a gauge or sparkline.
An unsupported metric is either omitted or explicitly says `unavailable`; it
is never drawn as zero. A rate is shown only after two valid monotonic samples
with a known interval. Counter reset, interface disappearance, overflow, and
sampling failure produce an unavailable sample rather than a spike.

- Wide: `CPU | RAM | Build FS | Read | Write | RX | TX`
- Medium: `CPU | RAM | Build FS | I/O`, where I/O is a textual aggregate with
  separate read/write values and never a fabricated combined counter
- Narrow: omit the strip and expose a compact summary in System Status or the
  Inspector

The strip's render-area breakpoints are explicit: `112+` columns is Wide,
`64..111` is Medium, and below 64 is Hidden. It requires at least four rows,
and Dashboard/Tasks allocate the bounded eight-row tier only when their
Workspace body is at least 46 rows high and at least one metric group is
authoritative. Otherwise those rows return to higher-priority workspace
content. Wide preserves the stable cell order and omits the paired Read/Write
or RX/TX cells when neither a current sample nor retained valid history proves
that optional host source. Medium similarly omits its two-line `I/O` cell when
disk rates are unsupported. Vertical separators and semantic graph roles are
shared across themes; no-color and reduced-motion retain the same text.

Zooming the Tasks `Workspace/Context` subfocus keeps a bounded Job History at
the top and gives the remaining body to expanded telemetry charts. CPU, RAM,
disk read/write, and network RX/TX each retain an independent semantic role and
an exact current-unit label; the build-filesystem gauge remains distinct when
its configured path exists. Every chart uses at most the latest 60 valid
samples. A missing current sample with retained history says `partial` and
keeps the older trail visible, while a wholly absent series says `unavailable`;
neither becomes zero. Genuine zero samples remain numeric. When height is too
small for both tiers, telemetry takes the body and each series collapses to its
text before its graph. The same Context identity and breadcrumb survive wide,
medium, and narrow zoom, and restoring zoom preserves history selection and
all workspace state.

CPU, RAM, and Build FS retain their cells when sampled. Disk I/O and network
cells appear only on supported hosts. The strip stores and displays bounded
history only. It does not increase redraw frequency: new points arrive on the
telemetry sampling cadence, and reduced motion disables any presentation-only
animation.

When a CPU, RAM, or Build FS strip cell has at least 16 columns and six rows,
it uses the M21 semicircular dial composition: a centered metric title, a
left-to-right colored utilization arc with a muted remainder, the exact whole
percentage inside the dial, and one centered context line below it. Unicode
uses diagonal and heavy horizontal arc segments; ASCII mode preserves the same
shape with `/`, `-`, and `\\`. The arc uses foreground styling only and must not
paint a rectangular filled background. Shorter cells retain the compact
horizontal terminal-gauge fallback described below so responsive layouts do
not lose authoritative values.

The CPU dial is titled `CPU Usage`; its context is the authoritative logical
core count when known and `utilization` otherwise. The compact fallback has a
numeric label in every determinate presentation. At 28 or more cell columns
the fallback label is `CPU n% · N cores`; at 16–27 columns the authoritative
core count contracts to `Nc`; below 16 columns the gauge retains `CPU n%` and
omits the lower-priority core count. A missing CPU sample renders
`CPU ! unavailable`, never `0%` or a perpetual activity claim. Normal
utilization uses the semantic CPU-graph role, while warning/error thresholds
use their semantic roles; no-color retains shape, text, and attribute
distinction. The dial and compact fallback are determinate and unchanged by
reduced-motion mode.

The RAM cell derives whole used percent with overflow-free integer arithmetic
from valid total/available byte samples. Its dial is titled `RAM Usage` and the
context line is `used/total unit`. In the compact fallback, at 38 or more cell
columns it labels the gauge `RAM n% · used / total`; at 28–37 columns the values
share the total's largest binary unit as `used/total unit`; below 28 columns the
gauge retains `RAM n%`. Capacity labels use at most one decimal binary-unit digit and
do not imply a fractional percentage. Missing fields, zero total, or available
greater than total renders `RAM ! unavailable`, never a synthetic capacity or
`0%`. Normal memory utilization uses the semantic memory-graph role and high
pressure uses warning/error roles. No-color and reduced-motion preserve the
same determinate text.

The build-filesystem cell is valid only when the configured build directory
and a consistent total/available `statvfs` sample are both present. Its dial is
titled `Build FS Usage` and the context line is the exact available capacity
followed by `free`. In the compact fallback, at 52 or more cell columns the label is
`BUILD FS n% · free/total unit free · <build-dir>`; at 34–51 columns it omits
the path but retains free/total; at 16–33 it retains `BUILD FS n%`; below 16 it
contracts to `FS n%`. The path is the configured build directory, not an
inferred block-device name. Missing context, missing fields, zero total, or
available greater than total renders `BUILD FS ! unavailable`, never `0%`.
Normal utilization uses the semantic progress role and pressure thresholds use
warning/error roles; no-color and reduced-motion preserve determinate text.

Disk read and write occupy separate rows/cells and always label the current
value in binary bytes per second (`B/s`, `KiB/s`, `MiB/s`, or `GiB/s`). Each
sparkline scales independently to the maximum of its own retained valid
history and uses the semantic disk-read or disk-write graph role. At 28 or
more columns labels are `Read` and `Write`; below 28 they contract to `R` and
`W`, and below 18 the current text takes priority over the graph. When the
current delta is unavailable the label says `! unavailable`; any older valid
trail may remain visible but is not presented as current. A real monotonic
zero delta renders `0 B/s`. First observation, reset, device change, overflow,
zero interval, or unmatched device appends no point, so none can create a
synthetic zero or spike. Reduced motion does not animate the graphs.

Network receive and transmit occupy separate `RX` and `TX` rows/cells and
always label the current value in binary bytes per second. Each sparkline
scales independently to the maximum of its own retained valid history and uses
the semantic network-RX or network-TX graph role. Below 18 columns current text
takes priority over the graph. When the current delta is unavailable the label
says `! unavailable`; an older valid trail may remain visible but is not
presented as current. A real monotonic zero delta renders `0 B/s`. First
observation, reset, interface change or disappearance, overflow, zero interval,
or absent lowest-metric IPv4 default-route interface appends no point, so none
can create a synthetic zero or spike. Hosts without a supported selected
interface keep both optional rows explicitly unavailable, and reduced motion
does not animate the graphs.

##### Telemetry provenance audit

The typed provenance catalog is authoritative for whether a metric may render.
The client host sampler runs at a nominal one-second cadence only while a build
or another managed operation is active. CPU, RAM, disk-I/O, and network
histories therefore retain the latest 60 valid samples per metric, not an
unconditional 60 wall-clock seconds; the caption is `60-sample history`.
Missing samples do not append zeroes.

| Metric | Authority and units | Precision/cadence | Unsupported or unavailable behavior |
| --- | --- | --- | --- |
| Host CPU | aggregate `/proc/stat` counters; whole percent | delta of non-idle/total counters, truncated; nominal 1 s active-operation sampling; 60 valid samples | Linux only; first sample, reset/non-increasing interval, read, or parse failure is unavailable |
| Logical CPU count | Rust `available_parallelism`; logical processors visible to the process | current runtime query at host-sample cadence; no history | show unknown when the runtime query fails or cannot fit the typed count |
| RAM | `/proc/meminfo` `MemTotal` and `MemAvailable`; bytes | reported kB multiplied by 1024; used percent derived from valid total/available; 60 valid samples | Linux only; omit unless both fields exist, total is nonzero, and available does not exceed total |
| Build filesystem | `statvfs` on the configured build directory; available and total bytes | `f_bavail`/`f_blocks` times fragment size at host-sample cadence; no history yet | Unix only; explicitly unavailable for an invalid/missing path or failed sample |
| Load 1/5/15 | `/proc/loadavg`; thousandths of a load unit | at most three decimals, nominal 1 s active-operation sampling; no history | Linux only; all three values become unavailable on invalid input |
| Disk read/write rates | `/proc/diskstats` counters for the exact device ID backing the configured build directory; bytes/s | read/write sectors multiplied by 512, then monotonic counter deltas divided by the measured interval; nominal 1 s active-operation sampling; 60 valid samples per direction | Linux only; first sample, reset, device change, overflow, zero interval, missing path, or a filesystem device absent from diskstats is unavailable and appends nothing |
| Network RX/TX rates | `/proc/net/dev` counters for the active lowest-metric IPv4 default-route interface; bytes/s | per-interface monotonic counter deltas divided by the measured interval; nominal 1 s active-operation sampling; 60 valid samples per direction | Linux only; first sample, reset, interface change/disappearance, overflow, zero interval, absent IPv4 default route, read, or parse failure is unavailable and appends nothing |
| Daemon connection | client replica status; lifecycle | event-driven Current/Stale/Synchronizing/Disconnected | render the exact replica state, never a numeric stand-in |
| Daemon uptime | daemon start wall-clock difference; seconds | saturating whole seconds, published nominally each second | unavailable until current daemon telemetry arrives |
| BitBake state | current daemon journal snapshot; lifecycle | event-driven exact lifecycle | stale/disconnected client state is named and old authority is not presented as current |
| Connected clients | current daemon snapshot client inventory; count | exact inventory length | unavailable unless the replica is current |
| Terminal sessions | current daemon snapshot PTY inventory; count | exact inventory length | unavailable unless the replica is current |
| Active jobs | daemon telemetry count of Connecting, Running, and Stopping jobs | whole count, nominal 1 s | unavailable until current daemon telemetry arrives; retained terminal jobs are not counted as active |
| Daemon queue depth | existing daemon telemetry wire field; count | currently mirrors connected-client count and is not a work queue | non-renderable; System Status must not label it as queue depth |
| Daemon resident memory | `/proc/self/statm`; diagnostic bytes | resident pages currently use an assumed 4096-byte page | non-renderable as precise memory until the runtime page size is authoritative |

The audit does not authorize per-task CPU/ETA or daemon version/PID. Disk and
network rate collection is host-optional: overlay/container filesystems may
not map to a diskstats device, and hosts without an active IPv4 default route
have no selected network interface. Those cases stay explicitly unavailable.

#### Responsive table columns

Column helpers select complete columns before row construction; renderers do
not format data that will be hidden. Column priority is stable:

| Table | Wide priority order | Medium | Narrow |
| --- | --- | --- | --- |
| Tasks | Task, Recipe, State, Elapsed, Progress, Worker, PID | Task, Recipe, State, Progress, Elapsed | Task, State, Progress |
| Jobs | Status, Operation, Target/context, Started, Finished, Elapsed, ID | Status, Operation, Target, Elapsed | Status, Target, Elapsed |
| Logs | Severity, Recipe, Task, Message, Source | Severity, Recipe, Message | Severity, Message |

Worker and PID render only when authoritative. Per-task CPU is currently not
part of `TaskInfo` and is omitted. An ETA is labeled `estimate` and renders
only when the model supplies or honestly derives it from completed work and
elapsed time; an unknown total or zero completed work renders `--`. PN, PV,
PR, workdir, daemon version, disk I/O, and network rates are not inferred from
recipe labels, paths, or the raster reference.

For the Tasks table, the render-area thresholds are explicit: below 84 columns
show `Task | Status | Progress`; 84–109 columns add `Recipe | Time`; 110 or
more may add `Worker | PID` when at least one retained typed row supplies each
field. The canonical reviewed 89×17 table retains its approved proportional
geometry. The responsive helper selects the complete column set before cells
are constructed, so hidden Worker/PID values are not formatted. An unselected
active row uses the running role across the full row; selection remains a
separate full-row treatment and the status cell retains its textual marker.

#### Focus and scroll presentation

Exactly one pane or modal owns focus. Focused panes use the semantic focused
border and, where applicable, one selected row. Unfocused selected rows retain
identity but use the inactive-selection treatment. No-color uses attributes,
not color, for the same distinction. Dialog and command-palette focus replaces
pane focus visually until the modal closes.

Every bounded scrollable region exposes position when content exceeds its
viewport. Titles use `top`, `1/N`, `N/N`, or an equivalent bounded range plus
`↑`/`↓` availability. Horizontally scrolled content exposes `←` and/or `→`.
Indicators are derived from retained length, viewport, selection, and offset;
they do not imply access to evicted content. Follow mode pins the log indicator
to the retained tail, while paused mode preserves its exact position.

#### Empty, loading, unavailable, and error states

All workspaces use the same textual state grammar:

- empty: `∅ No <items>.` followed by a real next action when one exists
- loading: `… Loading <items>.` with stable reduced-motion text
- unavailable: `! Unavailable — <exact typed reason>.`
- partial/degraded: `! Partial — <limitation>.` while retaining valid rows
- error: `✕ <operation> failed — <typed summary>.` followed by retry/open-log
  guidance only when that action exists
- stale/disconnected: name the stale authority and the reconnect or refresh
  action; never present stale data as current

An empty successful inventory is distinct from unavailable, failed, loading,
and filtered-to-zero. A filtered empty state keeps the query visible and
offers the real clear-search action. No state is encoded only by glyph or
color, and every progress visualization includes a numeric or textual
equivalent.

#### Concept-image adaptations

The raster concept controls hierarchy, density, proportions, spacing, borders,
selection, typography, gauges, meters, sparklines, status presentation, and
terminal-native tone. It does not authorize data. The following illustrated
items are adapted until authoritative support exists:

- per-task CPU and per-task ETA columns are omitted unless typed task data is
  added; host CPU is not attributed to a task
- PN/PV/PR, section, workdir, and log file render only from explicit metadata,
  never by parsing a recipe label or path
- disk and network throughput renders only from the bounded reset-aware host
  sampler; unsupported or currently unavailable sources remain omitted
- daemon version and PID render only if the daemon protocol supplies them
- illustrative action names and F-key assignments are replaced by Yoctui's
  actual typed keymap and capability availability
- decorative Navigator rows or badges without an authoritative workspace,
  inventory, job, warning, or error count are omitted

These omissions are honest unavailable behavior, not missing permission to
invent equivalent values.

#### M21 concept-screen acceptance

The six PNGs under `docs/design/m21/concepts` define reviewed visual direction,
not terminal pixels. A production-renderer acceptance catalog covers the same
six scenario identities at the canonical `160x50` size: idle Dashboard, active
Tasks, failed Errors, Images/rootfs composition, editor/application menu, and
terminal sessions. Each catalog fixture is assembled only from typed
`yoctui-model` state, uses the fixed `19:28:27` clock, calls the public
`yoctui_ui::render_at` path, checks scenario-specific semantic anchors, and
serializes every resulting Ratatui cell symbol and style into a reviewed
golden.

The catalog is also an executable gap ledger. A scene may initially retain the
closest truthful existing Yoctui projection, but its manifest must name every
missing concept capability and the exact incomplete registry task that owns
it. Verification fails when a gap references an unknown task, when a completed
task still owns a declared gap, or when a scene lacks its real-renderer golden.
Implementing tasks update the fixture, anchors, gap ledger, and reviewed golden
in the same commit. This lets the baseline pass without misrepresenting a
placeholder as completed concept parity.

Generated PNGs are never decoded or pixel-compared by the Rust UI tests. Their
hashes protect design provenance only. Exact regression authority remains the
production Ratatui cell buffer; a later pinned-font rasterizer may produce
deterministic PNGs from that buffer. Live PTY capture remains separate evidence
for terminal lifecycle and escape delivery.

#### Reusable rendering primitives

All workspaces use the shared render-only primitive vocabulary. A pane shell
owns border/title/base styles and resolves focused versus inactive borders. A
selected-row helper distinguishes the focused selection from an inactive
retained selection. Section headers and separators accept already-resolved
text and semantic styles. Status labels always include a textual marker.
Empty, loading, unavailable, partial, and error views follow the state grammar
above. Scroll indicators clamp offset and viewport to retained bounds.
Responsive columns add mandatory columns first and optional columns in stable
priority order without reordering their table positions.

These helpers own no model state and receive no backend/protocol values. A
workspace remains responsible for selecting the correct typed content and
semantic role; the helper only renders it.

---

## 3. Header

The header is always visible unless the terminal is below the supported
minimum. It occupies the shell's two-row bordered region and renders one
content row. Identity/build context is left aligned; daemon/BitBake health is
right aligned in a separately measured rectangle so health never overwrites
the higher-priority build identity.

The left-to-right priority order is:

1. `yoctui v<workspace-version>` identity, compiled from the package version
2. project identity
3. build state, including a non-color marker
4. selected build target
5. authoritative `MACHINE`
6. authoritative `DISTRO` and Yocto release

Project identity is the basename of the typed source directory, falling back
to the typed build directory. It is never copied from the build target. When
neither path is known it says `unavailable`; a missing target says
`not selected`. `MACHINE`, `DISTRO`, and release are omitted when absent and
are never inferred. Long project and target values are bounded before layout.

The right side always reports daemon replica state. BitBake lifecycle is shown
only when space permits and is authoritative only while the daemon replica is
Current. A stale, synchronizing, or disconnected replica forces BitBake to
`Unavailable` rather than exposing retained lifecycle state. Both health
labels use semantic theme roles plus the shared textual markers, so no-color,
high-contrast, and reduced-motion modes retain their meaning.

Header width tiers are deterministic:

| Terminal width | Left context | Right health |
| --- | --- | --- |
| `180+` Full | Yoctui, project, build, target, MACHINE, DISTRO/release | verbose daemon and BitBake |
| `150..179` Wide | Yoctui, project, build, target, MACHINE | verbose daemon and BitBake |
| `130..149` Wide compact | Yoctui, project, build, target, MACHINE | compact `D:` and `BB:` labels |
| `100..129` Medium | Yoctui, project, build, compact target | compact `D:` and `BB:` labels; MACHINE and DISTRO/release hidden |
| `80..99` Narrow | Yoctui, build, compact target | compact daemon only; project, MACHINE, DISTRO/release, and BitBake hidden |

Task counts, warnings/errors, progress, elapsed time, sstate reuse, and host
telemetry belong to the Tasks summary, Inspector, System Status, or telemetry
strip; the header does not duplicate them. Backend names, session numbers,
daemon versions/PIDs, and other unavailable identities are likewise not
fabricated merely to fill the visual target. Rendering an empty or undersized
header rectangle is a no-op and no supported tier may panic or overlap.

The version remains adjacent to `yoctui` at every supported header width. It
is the exact Cargo workspace/package version of the running binary, never a
runtime guess or daemon value. Repository verification requires that version
to increase for every commit and requires all internal path dependency
constraints to match it.

---

## 4. Navigator

The left pane is the primary workspace navigator. The canonical wide Tasks
workbench uses a mixed project-context tree, matching an IDE project explorer
rather than presenting fake abstract files. Its required top-level order is:

- `Layers`: configured layer inventory from typed workspace metadata
- `Recipes`: useful typed recipe shortcuts, including the active recipe and
  common image/recipe entries when available
- `Images`: typed image targets and discovered artifacts
- `Tasks`: Build, Test, QA, Devtool, Wic, SDK, Security, and Utility
- `Targets`: active machine and other typed targets when available

Top-level rows use an expanded/collapsed tree glyph and amber semantic accent.
Children are indented and use folder or disclosure glyphs when Unicode/icons
are enabled. The selected child uses the complete available row width. A
bounded footer inside the Navigator reports selected layer, selected recipe or
job identity, and process/build identity only when those typed values exist.

The project-context tree is a navigation projection over existing typed model
state. It does not parse BitBake output, fabricate reference entries in
production, or replace the complete destination catalog. Other workspace
states, and compact layouts where the contextual tree would hide
functionality, render the complete workspace rail in stable `OVERVIEW`,
`CONTENT`, `BUILD`, `VALIDATE`, and `TOOLS` groups. This distinction prevents a
workspace-owned layer, recipe, package, or artifact tree from being duplicated
as a fake filesystem in the Navigator. Entries that do not fit remain
reachable through bounded scrolling, `F10 Menu`, the command palette, or their
documented global shortcut.

Each rail group heading and each destination is rendered exactly once per
frame. A destination must never be repeated as both a synthetic group label and
its real child. A host-terminal resize invalidates the complete terminal
backend buffer before the next frame so cells from the previous geometry cannot
survive as duplicate Navigator rows or workspace titles.

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
- Compatibility
- Settings

The currently active workspace is highlighted.

The workspace rail shows a badge only when its value is authoritative. The
currently supported badges are active typed task count, aggregate build error
count, live/paused log-follow mode, and retained typed Devtool status count.
Unsupported illustrative badges are omitted:

```text
Tasks          12
Errors          3
Logs          LIVE
Devtool         2
```

Expanded groups use `▾`; collapsed groups use `▸`. `Left` collapses the group
that owns the selected destination. `Right` expands it, or activates the
selected destination when already expanded. Keyboard movement skips hidden
destinations. When the rail is taller than its viewport, the title reports the
bounded selected-row position as `current/total` and keeps selection visible.

Mouse behavior uses the same typed actions: the wheel moves bounded selection,
a group-heading click toggles that group, the first destination click selects
and focuses it, and a repeated click activates it. The canonical Tasks
project-context rows select their owning real workspace; they never simulate a
filesystem operation.

Keyboard:

- `j` / `Down`: next entry
- `k` / `Up`: previous entry
- `Enter`: activate entry
- `Left`: collapse the current branch or move to its parent
- `Right`: expand the current branch or activate its selected child
- single-letter global shortcuts may jump directly to common workspaces
- `Tab`: move focus to workspace
- `Ctrl+B`: enter the configurable terminal-session prefix layer; the default
  second-key map is `c` create, `n`/`p` next/previous session, `%` horizontal
  split, `"` vertical split, `x` close pane, `d` detach, `:` command palette, `?` help, and
  `o` take PTY writer control
- the prefix waits at most one second, shows its pending state in the footer,
  and `Ctrl+B Ctrl+B` sends a literal prefix byte to the terminal

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
- contextual rails include the next `Tab` destination when it fits; the
  focused border/selection treatment and Help expose the complete forward and
  backward focus map when that lower-priority hint is omitted
- arrow keys affect only the focused region
- pane focus consumes only keys mapped to focus or pane navigation; every
  unmatched key continues through the active workspace and global shortcut
  routes instead of being discarded
- global actions such as `Ctrl+P`, `F1`, `F10`, `q`, and `Ctrl+C` remain
  reachable from Navigator, Workspace, and Inspector focus
- a non-dialog notification consumes only its documented `Enter` activation
  and `Esc` dismissal keys; unrelated input continues through normal routing
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

The focus-flow acceptance sequence dispatches real `Tab`/`Shift+Tab` actions
through Navigator → Workspace → Inspector in both directions and renders each
state at `160x50`, `100x30`, and `80x24`. It also proves dialogs and command
palette retain and restore the exact previous pane, prefix commands leave
terminal-session shell focus unchanged, and resizing/focus cycling does not
mutate Navigator or workspace selections.
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
- `i`: Git details
- `m`: metadata view
- `d`: dependencies view
- `Esc`: return to the configured-layer inventory

The tree must not eagerly scan the entire Yocto source tree.

---

## 8. Inspector

The right pane is context-sensitive.

Every Inspector title is `Inspector: <typed mode>` rather than the generic
word alone. The model-owned modes currently distinguish Navigator,
Daemon/session, Task, Job, Dependency, Signature, Recipe, Package, Artifact,
Test, Security, QA, Layer, File, Configuration, Utility, Log, Error, Help,
Build environment, Compatibility capability, and Settings. A selected regular
layer-tree entry changes Layer to File; Navigator focus changes the mode to
Navigator without changing workspace selection.

The shared document order is fixed and sections with no authoritative content
are omitted:

1. `PRIMARY FACTS`
2. `SECONDARY FACTS`
3. `RELATED PATHS`
4. `RECENT OUTPUT`
5. `CONTEXTUAL ACTIONS`
6. `SYSTEM / COMPATIBILITY`

Section headings use the semantic heading role. The pane shell owns the single
focus border in wide, overlay, and narrow modes; child sections do not invent
independent focus. Tasks retains separate bounded subpanes because its live log
tail and actions need independent height priority, but their names and order
match this grammar. Related paths come only from typed recipe, layer/file, log,
error, image/SDK artifact, or job context fields. Logs place the selected
message under Recent output rather than duplicating it among facts.

Contextual actions use one shared action-list grammar. Expanded rows are
`<marker> <aligned action name> [<shortcut>] — <state>`; compact rows preserve
the same fields without padding. `✓` identifies an enabled local/available
action, `~` an enabled limited action, and `×`, `?`, or `!` identify disabled,
unknown, or unsupported actions. State therefore never depends on color.
Enabled names use primary text and semantic accent shortcuts; disabled rows use
the disabled role and no-color dim emphasis. An exact typed `Reason:`, each
limitation, and the selected implementation follow the affected row when
present. Renderers consume the closed workspace action inventory and do not
invent labels or bindings. The app keymap owns dispatch, including `B` for
Build options; the CLI does not keep a second direct-key definition.

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

The implemented operational hierarchy renders Current Build first from the
shared progress projection, followed by the capability-aware next action,
active tasks, retained attention, recent jobs/artifacts, environment health,
and telemetry. `F2`, `l`, `e`, `F3`, `F8`, `E`, and `M` open the existing typed
Tasks, Logs, Errors, History, Images, Build Environment, and Maintenance
workflows; the dashboard does not duplicate their reducers or bypass their
availability and confirmation boundaries. Missing build-filesystem or sstate
authority remains explicitly unavailable. Compact layouts retain the same
state and routes in an Operational Summary, and a short Dashboard Inspector
prioritizes system health over repeating the complete contextual-action list.

The Dashboard's Workbench Center is the one-stop cross-workspace summary. It
shows at most three recent typed contexts, active background jobs, Raw favorite
commands, and daemon terminal sessions alongside the Dashboard's bounded next
action, failures, recent work, and artifacts. The selected terminal projects
first; Raw favorites preserve their persisted order and current
available/limited/unavailable/unknown/unsupported or stale state. Every row
names its owning route: `F2` Tasks, `F3` History, `e` Errors, `F8` Images,
Dashboard-scoped `f` Raw Favorites, and Dashboard-scoped `t` Terminal Sessions.
These rows are
summaries, not activation cards: keyboard, menus, and mouse continue through
the owning typed workspace and its ordinary availability or confirmation
boundary. Empty, disconnected, stale, and unavailable sources remain explicit.
Compact layouts preserve one line for context, active work/attention, artifact,
favorite, and terminal state under an Operational Command Center title.

### Guided workflow onboarding

The Help menu and command palette expose one focus-trapped **Workflow guide**
overlay. A legacy or new session with no saved guide state opens it once on
first interactive startup; dismissal prevents later automatic reopening, while
the Help route always resumes the saved cursor. Opening, resuming, selecting,
or dismissing the guide must not start a build, scan, shell, terminal session,
or other process.

The guide has exactly six typed steps in this order: verify Build environment,
select an image target, review the first Build options confirmation, inspect
Logs/Errors, explore Images/Rootfs evidence, and learn Terminal Sessions. Each
row names its existing authoritative destination and displays one textual
state: `COMPLETED`, `CURRENT`, `BLOCKED`, `SKIPPED`, `STALE`, or `UNAVAILABLE`.
Markers `[x]`, `[>]`, `[-]`, `[~]`, `[!]`, and `[?]` preserve the same meaning
without color, Unicode, or motion. A formerly completed step becomes `STALE`
when its current typed evidence or an earlier prerequisite disappears; later
steps cannot silently remain actionable through that stale prerequisite.

`Enter` explicitly opens the selected destination through its existing typed
route. Target selection remains a picker, the first build remains a review and
confirmation dialog, Images retains its correlated acquisition boundary, and
opening Terminal Sessions does not create a shell. `n` advances only when the
current step's exact evidence is satisfied, `s` explicitly skips it, `r`
restarts the guide, arrows or `j`/`k` change the inspected row, and `Esc`/`q`
dismisses. Progress, skips, cursor, completion, and dismissal persist together
in the existing bounded private `session.toml`; invalid or future state is
rejected without replacing the prior file. Wide layouts may use additional
spacing for a low-density reading mode, but narrow `80x24`, no-color, and
reduced-motion layouts retain every state and control in ordinary terminal
text.

On terminals with enough vertical space, Dashboard includes a dedicated
terminal-native telemetry cockpit. It renders determinate CPU, memory, and
build-filesystem gauges; bounded CPU and memory sample-history sparklines;
logical CPU count; 1/5/15-minute load averages; and exact used/total byte
labels. Missing or unsupported metrics remain visibly unknown. Gauges clamp
only for rendering and never manufacture samples. At the minimum supported
size the cockpit collapses to compact labeled values so build and task state
remain visible.

Build progress surfaces average completed-task velocity and an ETA only when a
start time and a nonzero authoritative total make those values meaningful.
They are labeled as averages/estimates, use bounded finite arithmetic, and
remain unknown otherwise. Determinate task bars use fractional Unicode blocks
for sub-cell resolution while unknown tasks retain the existing honest
activity animation.

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

`B` opens the image build-options dialog. A lower-case `b` remains a
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

At wide widths it is a three-tier cockpit matching the persistent workbench:

1. `Tasks: <target>` table with Task, Recipe, Status, Time, and Progress.
2. `Log Viewer — <selected task>` showing bounded matching typed log entries.
3. `Job History` showing retained background jobs/build records with identity,
   name, type, status, timestamps when available, and elapsed time.

The table is the primary region and receives at least 40% of available height.
The log viewer and history remain visible when height permits. At reduced
height they collapse in that order to preserve task selection and controls.
Overall build progress and active filters move into the table title/status
rows instead of consuming large standalone cards.

The first two bordered content rows are the compact build summary. With a
known nonzero total, row one is a strong determinate gauge labeled with integer
percent and bounded `completed/total`. Without that authority it is plain text
`progress unknown` plus the stable/animated activity marker and `completed/?`;
an empty gauge or `0%` is forbidden. Row two shows typed Active, Waiting,
Warnings, Errors, and Elapsed values. At 96 columns it may append the honestly
derived average rate and `ETA`; compact widths use `A/W/!/✕` labels. Terminal
elapsed time freezes from the retained build record. Yoctui currently receives
no authoritative sstate-reuse percentage, so this summary omits it rather than
parsing logs or estimating it.

Task lifecycle presentation has seven exact text-and-marker labels:

| Model meaning | Label | Non-color meaning |
|---|---|---|
| identified task announced by `TaskQueued` | `· Queued` | pending, dim |
| unobserved work derived from the build total | `○ Waiting` | pending, bold |
| executing task announced by `TaskStarted` | `▶ Running` | running, bold |
| successful terminal task | `✓ Succeeded` | success, bold |
| failed terminal task | `✕ Failed` | error, bold and underlined |
| explicitly cancelled task | `■ Cancelled` | warning, bold |
| task whose backend lifecycle was lost | `? Lost` | error, bold and underlined |

The words are stable ASCII text and carry the meaning if a terminal substitutes
a marker glyph. No-color changes only the resolved colors, never the marker or
word. Selection replaces the unselected row treatment with the single visible
selection treatment, while the State cell retains its label. Reduced motion
changes only indeterminate activity, not lifecycle labels.

The task table renders only the rows that fit its bordered viewport. The
viewport follows `task_progress_scroll` and always includes the selected row;
off-screen retained tasks are not formatted into Ratatui rows on every frame.

Job History is one ordered view over two authoritative retained sources:
background jobs followed by completed build records. Nonterminal background
jobs are pinned first, newest first, then terminal background jobs and retained
build records appear newest first. Its lifecycle labels are `· Queued`,
`… Starting`, `▶ Running`, `! Cancelling`, `✓ Succeeded`, `✕ Failed`,
`■ Cancelled`, and `? Lost`; markers plus words preserve meaning without color.
The responsive column contract is:

- below 84 columns: Status, Operation, Target/Context, Elapsed
- 84 through 117 columns: add Type and Started
- 118 columns and wider: add ID and Finished

Missing timestamps and context render as `--` or `unavailable`; the renderer
does not reconstruct them from logs. The standalone Job History workspace keeps
all active rows visible ahead of the scrollable terminal portion whenever its
viewport permits. Keyboard selection ranges over the combined ordered view and
drives a bounded detail panel containing typed operation, type, exact state,
context, known times, warning/error counts, outcome, and latest retained output.
An empty view explicitly states that no jobs or build records are retained.

Both Job History surfaces include the same compact background-job summary.
`Active` counts Starting, Running, and Cancelling jobs; `Queued` remains
separate; `Failed` counts only the exact Failed state; and `Recent complete`
counts every retained terminal background job. Completed build records are not
added because a build may already have a background-job record. `Daemon-owned`
is shown only while the client has a Current daemon replica and is the exact
number of summaries in that replica; stale, synchronizing, and disconnected
replicas omit the count. At widths below 84 columns the stable textual form is
`A<n> Q<n> F<n> Done<n> [D<n>]`; it remains text rather than color-only state.

The wide Tasks Inspector is subdivided into titled sections:

- primary task facts
- secondary facts, paths, and dependencies
- recent matching log tail
- available typed actions and their actual shortcuts
- daemon/BitBake/job/session/system status

These sections render only authoritative state. Actions that are unavailable
remain absent or explicitly disabled; labels never imply an unimplemented
command.

The Tasks action list orders Cancel active build, Open Logs, Build History, and
Build options. Their authoritative shortcuts are `c`, `l`, `h`, and `B`.
Cancel is disabled with `No active build can be cancelled.` outside an active
build lifecycle; environment-backed actions additionally retain their exact
compatibility reason. The F2 task-inventory route remains in the global keymap
but is not repeated as an action inside the already-open Tasks Inspector.

For a selected task, primary facts show Task, Recipe, PN, PV, PR, exact state,
and progress. PN is the task's typed recipe identity. PV is the matching
authoritative recipe-inventory version when present; it is `unavailable` when
that inventory has no match. The model currently has no task-specific PR or
workdir authority, so both remain explicitly `unavailable` rather than being
inferred from a generic environment variable or log path. Secondary facts show
worker, PID, start time, elapsed time, task log path, and the exact typed task
dependencies. Missing values use the shared unavailable treatment.

Recent output contains only retained log entries whose recipe and task both
match the selection. The model bounds the tail to the requested visible
capacity before the UI renders it, and the displayed entries remain in
chronological order. The aggregate waiting row reports only its count and does
not inherit metadata or a synthetic percentage. Wide mode shows all sections;
short wide/overlay layouts remove system status and then recent output before
primary facts, context, and actions; narrow mode exposes the same Inspector
through the pane switcher without changing its authority.

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
- determinate task rows render a bounded bar and integer percentage in both the
  Dashboard task panel and Tasks workspace
- fractional backend percentages are normalized before reaching UI state;
  widgets never parse, round, or otherwise repair raw backend values
- PID-only progress is shown only after the backend correlates it with an
  authoritative task-start identity; unmatched progress remains absent

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

When BitBake reports a total without individual task identities, the
workspace shows one honest aggregate waiting row. It must not invent recipe,
task, worker, or timing metadata for those waiting tasks. The Inspector labels
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

Fractional, negative, non-finite, boolean, or otherwise malformed backend
progress must never break the typed event stream. The normalization adapter
converts finite non-negative percentages to bounded integers, preserves
negative/invalid values as unknown, and ignores PID-only task events that
cannot be correlated with an authoritative active task.

Every dropped or coalesced event count must be observable.

Loss of the daemon client connection immediately invalidates a nonterminal
build projection. Retained active and queued task rows become `Lost`, waiting
work becomes zero, and the build is labelled `Lost` rather than continuing to
look `Parsing` or `Running` without current authority. The interactive client
then retries a bounded local attach. A successful replacement snapshot is the
sole recovery authority and may restore the build as active or install its
terminal completion result. Ordinary client-local navigation and selection do
not change during this recovery.

An attached client never starts a second client-local BitBake metadata probe
during startup. Workspace source/build identity and any retained inventory come
from the daemon snapshot, including when the client shell itself was not
initialized. Missing optional inventory remains unavailable without changing
an Idle, successful, cancelled, or otherwise terminal build into Failed.

Client access origin checks both standard OpenSSH variables in priority order.
A malformed or empty higher-priority value does not hide a valid client IP in
the fallback variable; when at least one SSH variable exists but neither
contains a valid address, the origin is explicitly `SSH (unknown)`.

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
- exact source-path filter
- newest-retained-relative time-range filter
- retained-entry bookmarks
- source-path display
- open source log in editor
- copy selected line/details
- copy a bounded filtered export
- bounded retention and eviction counters

The workspace is split into a five-line `Log activity` section and a primary
`Log Viewer` section. The activity section keeps follow/pause and wrap state,
active severity/recipe/task/build filters, visible search query/result
position, exact source/time filter chips, bookmark count, retention pressure,
and currently enabled actions visible. The
Viewer title is:

```text
Log Viewer — <recipe:task|recipe|task|global|no selection> · <following|paused> · V <current/total> · H <offset/max|wrapped>
```

The same state also has one compact projection used by the activity row and
embedded task Log Viewer. It always begins with `▶ Following`/`▶ Follow` or
`Ⅱ Paused`, then conditionally appends `◆ Filtered`, `/ Search`, and
`! Evicted <count>`. At 96 columns of available label space it may include the
query, evicted warning/error counts, and coalesced count; compact labels retain
the state words and omit those lower-priority details. No inactive label is
shown, and no indicator depends on color or animation.

Vertical position is clamped to the filtered retained result set. Horizontal
position is a bounded character offset against the longest visible retained
message and becomes `wrapped` when wrapping disables horizontal movement. An
empty retained stream and a non-empty stream with no filter/search matches are
different explicit empty states. The log stream has no separate loading state:
backend connection/reconnect state remains in System Status, and typed error
records render as errors rather than a fabricated workspace-loading failure.

Rendering requests a model-owned viewport window and allocates references only
for the visible rows; it never collects the complete filtered result on a
frame. Counting may scan the bounded retained store, but a `v`-row Viewer owns
at most `v` row references. Source filtering compares the exact typed path.
Time ranges cycle `all`, `1m`, `5m`, and `1h`, anchored to the newest retained
typed timestamp so rendering needs no wall-clock mutation. Wide filter chips
show exact values; compact chips retain `R/T/B/S/I` plus `all`, `on`, or the
exact time range. The Inspector retains the full selected source path.

Every severity has a text marker (`· Trace`, `i Info`, `! Warning`, `✕ Error`)
as well as semantic styling. Incremental search highlights every visible
case-insensitive match with the semantic accent role; no-color uses bold and
underline attributes. Rendering consumes only the adapter-normalized retained
message. It never parses ANSI or raw BitBake output.

The selected log entry appears in the Inspector with full multiline content,
bookmark state, and metadata. `C Copy entry` copies structured retained data
through the existing typed clipboard effect, capped at 64 KiB with an explicit
UTF-8-safe truncation marker. `E Export view` copies a deterministic header,
loss counters, and filtered structured entries through the same typed effect,
capped at 256 KiB with included/omitted counts and an explicit marker.
`o Open source log` is
listed only when the selected entry contains an authoritative source path; the
action remains typed and is omitted otherwise.

`m` toggles the selected retained ID in a bounded bookmark set. `]` and `[`
jump to the next/previous retained bookmark with wraparound, temporarily
exposing an exact bookmarked ID without clearing user filters. Bookmarked
ordinary entries receive the same preferred-retention treatment as diagnostics,
but hard entry/byte bounds still win; eviction removes the stale bookmark and
increments ordinary or warning/error loss counters. Opening Logs from a
selected Task or Job History record jumps to the newest exact recipe/task or
build-correlated retained ID when one exists. Errors uses the same typed jump.

Controls:

- `↑`/`↓` or `k`/`j` selects an older/newer visible entry and pauses follow
- `f` toggles live follow; resuming selects the newest matching entry
- `w` toggles wrap; horizontal offset resets when wrap is enabled
- `←`/`→` scrolls horizontally only while wrap is disabled
- `/` starts incremental search; `Enter` or `Esc` finishes it
- `n`/`N` selects the next/previous search match
- `Ctrl+U` clears the retained log query without changing other filters
- `s`, `R`, `T`, and `B` cycle severity, recipe, task, and build filters
- `S` cycles exact retained source paths; `I` cycles the time range
- `m` toggles a bookmark; `]`/`[` select next/previous bookmarks
- `o` opens the selected source path in the configured editor
- `C` copies bounded structured selected-entry details and `E` copies the
  bounded filtered export when a supported clipboard tool is available

Retention prefers warnings, errors, cancellation records, disconnects, and
final results over ordinary informational entries. Repeated adjacent ordinary
entries may be coalesced. Evicted warning/error counts and the coalesced count
remain visible. If only protected records exceed a configured limit, eviction
is still bounded and explicitly counted.

### Yoctui self-diagnostics view

The Logs workspace begins with two textual views, `[BitBake logs]` and
`[Yoctui diagnostics]`; `v` switches between them and either label can be
clicked. They are separate typed authorities. A Yoctui tracing record never
enters `LogState`, gains recipe/task/build/source metadata, appears in Errors,
or participates in BitBake correlation. The diagnostic Inspector begins with
`Authority: local Yoctui tracing`; the BitBake Inspector begins with
`Authority: BitBake domain log`.

The Yoctui view captures only tracing emitted by the local interactive client
after tracing initialization. It does not claim that daemon-process tracing is
present. Each record retains a timestamp, `Trace`/`Debug`/`Info`/`Warning`/
`Error` level, exact tracing target, bounded formatted fields, and a stable
retained ID. Every level has a text marker (`·`, `◇`, `i`, `!`, or `✕`) in
addition to semantic styling.

Capture ingress is a 1,024-record nonblocking channel and the runtime drains at
most 256 records per frame. A formatted ingress event is capped at 64 KiB; the
independent model store then enforces its configured entry and byte bounds.
Queue loss and retention eviction are separate saturating counters and remain
visible even after `c` clears retained entries. Rendering uses an
`InternalLogWindow` with at most the visible row count, so a high-volume store
does not allocate every filtered row per frame.

The view owns independent follow/pause, selection, exact level and target
filters, and case-insensitive target/message query state. Empty retention and
filtered-empty are distinct. `E` copies a deterministic diagnostic export
through the existing clipboard effect, capped at 256 KiB with included,
omitted, ingress-loss, retention-loss, and truncation accounting. It has no
source-open, bookmark, BitBake filter, or domain-log copy actions.

Yoctui diagnostic controls:

- `v` switches back to BitBake logs
- `↑`/`↓`, `k`/`j`, Page Up/Down, Home, and End move bounded selection and
  pause follow
- `f` toggles follow and selects the newest matching record when resumed
- `s` cycles the level filter; `T` cycles exact retained tracing targets
- `/` starts query editing; `Enter`/`Esc` finishes and `Ctrl+U` clears it
- `E` copies the bounded filtered export; `c` clears retained diagnostics while
  preserving both loss counters

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
from the daemon-owned capability snapshot plus that typed status. Devtool
status, edit-recipe, modify, update-recipe, finish, deploy-target,
undeploy-target, reset, and upgrade are independently probed; one working
subcommand never enables another. Missing or stale capability authority shows
the retained exact reason and no Devtool process starts. A recipe outside the workspace may be modified but
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

Graph rendering degrades deterministically: terminals at least 130 columns use
a topology projection, 100–129 use a tree, and narrower supported terminals use
a compact table. All three retain the same typed relationship, stable source
position, selection marker, and partial/clipping facts. No-color uses ASCII
branches; the Inspector is the complete screen-reader text projection.

The center rows come only from normalized `DependencyGraphState` nodes and use
their deterministic model order. Each row shows recipe or task kind, its exact
identity, and incoming/outgoing edge counts. Build, runtime, and task edge
families are named in the Inspector; widgets never infer an edge from names,
logs, or provider paths. `↑`/`↓` or `k`/`j` changes the selected typed identity.
A linear-time adjacency index projects at most 8,192 rows and 64 levels;
cycles/cross-edges are reported without revisiting identities, and disconnected
authoritative nodes remain visible. Collapse, filtering, and row/depth clipping
report hidden counts.
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
- `←`/`→` or `h`/`l`: collapse or expand the selected identity
- `Space`: toggle selected expansion
- `/`: edit the bounded identity filter; `Ctrl+U` clears it
- `v`: toggle forward/reverse traversal, anchoring reverse view at the current
  typed selection so subsequent navigation remains stable
- `Tab`/`Shift+Tab`: use the global pane focus cycle
- `Esc`: return to Dashboard through the global action

Missing inventory entries, provider paths, or task logs leave the action inert
and show an exact notification. `o` never guesses a recipe file from layer
layout, and `L` never searches console text. Recipe nodes may expose a provider
but never a task log unless the backend explicitly supplies one.

State presentation is explicit:

- not loaded: explain that Recipes `A` starts dependency inspection
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

Mouse wheel uses the same typed selection reducer as keyboard navigation. A
primary click resolves the exact projected row identity in the app layer; the
stateless renderer never owns selection, expansion, filtering, or offsets.

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
At supported dimensions, authoritative identity/value fields, provenance,
overrides, and operations precede action-availability guidance so defining
sources remain in the visible detail region.

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

`E` opens a bounded, focus-trapping `Configuration.toml` editor prefilled from
that authoritative effective value as `value = "..."`. It follows the shared
Normal/Insert convention. `Enter` validates that one quoted TOML value,
escaping quotes and backslashes, then opens a separate confirmation dialog.
Newline and other control-character injection is rejected. The confirmation shows the exact
destination `build/conf/local.conf` and exact quoted assignment. Its `Enter`
revalidates the typed request, replaces the exact active variable assignment
or appends it when absent through a permission-preserving atomic rename, then
refreshes that exact global identity from BitBake. A write failure leaves the
file and prior detail untouched; a refresh failure retains the prior detail
and reports that the write succeeded. `Esc` from either dialog performs no
write and restores the exact prior pane. Editing never writes before the
second, preview-confirming `Enter`.

No silent edits.

The BBMASK edit shortcut opens the same bounded editor as `BBMASK.toml`, using
one `bbmask = "..."` value. Its validation and explicit write confirmation
remain separate from editing.

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

Tool availability, generated pkgdata, and each required command/option are
independent capability records. A missing tool, unsupported command, pkgdata
not yet generated, and a successful query with no rows have distinct messages.
Package actions remain disabled with the exact capability reason until the
current environment snapshot positively authorizes their complete argv.

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
through `I` import. `I` opens the bounded `Security import.toml` popup with
the `root` value selected and the shared Normal/Insert navigation, Home/End,
copy/paste, and persistent shortcut row. Typed path errors remain visible in
the popup. Import accepts one normalized absolute regular
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

`I` opens the bounded `QA import.toml` popup with the `root` value selected and
the shared Normal/Insert navigation, Home/End, copy/paste, persistent shortcut
row, and in-popup typed validation. It accepts one normalized absolute
canonical regular QA report or a bounded canonical directory. Only documented adapter
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
The bounded `Wic create.toml` popup contains:

- read-only machine identity
- typed image-target selection
- typed kickstart selection
- a normalized absolute output directory
- optional bmap generation
- a typed compression choice: none, gzip, bzip2, or xz

The initial document prefers the active image and configured `WKS_FILE` only
when both identities occur in the latest typed inventories. It follows the
shared Normal/Insert convention and uses named TOML fields. The machine line
is displayed for context but must exactly match the selected authoritative
image; kickstart names are resolved only against the latest typed inventory.
`Enter` validates the document and opens an exact shell-free argument preview
for cooked mode:

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

`c` opens the bounded `Sstate readiness.toml` popup only when the exact
readiness capability is available. The document exposes `targets`, `mode`,
`output`, `log`, and `timeout`; targets begin selected and empty, mode begins as
`isolated_tmpdir`, output and log paths begin absent, and timeout begins at 3600
seconds. Mode accepts only `isolated_tmpdir` or `same_tmpdir`. The shared popup
Normal/Insert navigation, Home/End, selection, copy/paste, undo, and shortcut
footer apply. `Enter` validates the whole document and requests the existing
typed adapter preview without running a command, while Normal-mode `Esc` or `q`
closes without side effects. Validation remains visible in the focus-trapped
popup.

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

`d` opens the bounded `Sstate cleanup.toml` popup only when the exact cleanup
capability and canonical `SSTATE_DIR` are available. Informational comments
show the read-only cache and stamps roots; edits to those comments cannot alter
the authoritative capability metadata. Native TOML fields expose `duplicates`,
`orphans`, `unreferenced_by_stamps`, and `jobs`; duplicates begins true, the
other modes begin false, and jobs begins at one. The shared popup can select
quoted strings, native booleans, and integers with `e`, and retains its
Normal/Insert navigation, Home/End, copy/paste, undo, and shortcut footer.
`Enter` validates and requests read-only candidate discovery; it cannot open
the deletion phrase or destructive confirmation until the adapter returns an
exact typed candidate preview. Normal-mode `Esc` or `q` closes without
discovery or deletion.

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

`e` opens `PR service export.toml` and `m` opens `PR service import.toml` only when the exact
native helper, initialized build directory, and configured PR endpoint are
available. The shortcut fixes the operation; the build directory and endpoint
are read-only informational comments, while the selected `file` TOML string
begins empty and accepts one canonical absolute `.conf` or `.inc` path. The
shared popup Normal/Insert navigation, Home/End, selection, copy/paste, undo,
and shortcut footer apply. `Enter` validates and requests an exact adapter
preview without running the helper; Normal-mode `Esc` or `q` closes without
side effects. The resulting typed preview shows the native server-stop and
cache-invalidation warning, labels import as changing PR data, and states that
export may replace the selected destination.

### 20.4 Release evidence

Locked-cache generation uses `gen-lockedsig-cache` with the exact ordered
inputs: locked-signature include file, input cache directory, output cache
directory, native LSB string, and optional filter file. Because matching
destination files may be replaced, the exact canonical output root and
replacement warning receive destructive styling and a separate explicit
confirmation. Completion returns a bounded inventory of created/replaced
evidence.

`l` opens the bounded `Locked cache.toml` popup only when the exact generator capability and
authoritative native-LSB metadata are available. Locked-signature include,
input cache, output cache, and optional filter begin empty; native LSB is
read-only context. The popup exposes `locked_signatures`, `input_cache`,
`output_cache`, and `filter` strings with shared Normal/Insert navigation,
Home/End, selection, copy/paste, undo, and shortcut footer. `Enter` validates
canonical absolute inputs and requests an adapter preview without running the
generator; Normal-mode `Esc` or `q` closes without side effects. The resulting
typed preview states that matching files beneath the exact output cache may be
replaced and retains the separate destructive confirmation.

Build-history comparison uses `buildhistory-diff` with one exact canonical Git
repository and zero, one, or two validated revisions, plus typed report-version,
report-all, signature, signature-diff, exclude-path, and no-colour choices.
Its report is replaceable, bounded, and retains both resolved revisions.
`build-compare` is a separate optional capability and is disabled when absent;
it is never emulated by relabelling `buildhistory-diff`.

`h` opens the bounded `Build history.toml` popup only when the exact
`buildhistory-diff` capability and authoritative canonical `BUILDHISTORY_DIR`
repository is available. Repository is read-only context. The document exposes
revision and comma-separated exclusion strings plus native TOML booleans for
report-version, report-all, signatures, signature-diff, and no-colour. Shared
Normal/Insert navigation, Home/End, selection, copy/paste, undo, and shortcut
footer apply. `Enter` validates and requests an exact adapter preview without
running a comparison; Normal-mode `Esc` or `q` closes without side effects.
Bounded session output and the separate unsupported `build-compare` interface
remain explicit in the typed workflow.

Git archival uses `oe-git-archive` with exact data and repository directories,
typed create/bare/tag choices, branch/tag/message templates, exclusions, and
notes. Push is never implicit. Local archive creation has an exact preview;
repository creation, tag replacement risk, or overwriting tracked output is
called out in confirmation. A requested remote push is a second network side
effect requiring a separate explicit confirmation after the local result.

`a` opens the bounded `Git archive.toml` popup only when the exact
`oe-git-archive` capability
is available. Data and Git directories begin empty. Create and create-tag begin
selected, bare begins clear; branch, tag, commit-subject, and tag-subject begin
as `release/{machine}`, `release/{tag_number}`, `Release {commit}`, and
`Release tag {tag_number}`. Commit/tag bodies, comma-separated exclusions,
comma-separated `reference=/absolute/file` notes, and push remote begin empty.
The document uses strings for paths, names, messages, exclusions, notes, and
remote, plus native TOML booleans for create, bare, and create-tag. Shared
Normal/Insert navigation, Home/End, selection, copy/paste, undo, and shortcut
footer apply. `Enter` validates and requests an exact adapter preview without
creating, tagging, or pushing; Normal-mode `Esc` or `q` closes without side
effects. A non-empty
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

The common visual contract is render-only and consumes the existing typed
dialog state. Every dialog outer shell uses the semantic background, focused
border, and heading roles and prefixes `modal ·` to its title so focus trapping
remains visible even when a long title is clipped. Confirmation/destructive
shells prefix `confirm modal ·`/`destructive modal ·`; danger is therefore never
communicated by color alone. Workspace renderers do not choose literal colors.

Dialog content follows this stable order where the type supplies each region:

1. concise body/operation identity
2. aligned facts or fields
3. bounded validation/unavailable reason
4. persistent controls

Field rows reserve a marker and aligned label: `▶` selected, one blank cell for
an enabled unselected field, and `–` disabled. Editing state is written as
`[editing]`; selected and disabled state never rely only on color. Validation
uses `✕ Validation: <exact reason>` in a reserved, wrapped area. It must not
replace the primary operation identity or the final controls. Buttons and
controls use bracketed key-first text such as `[Enter] Confirm` and
`[Esc] Cancel`; destructive confirmation says `[Enter] Confirm destructive`
or an equally exact typed operation. Dialogs that close on any key say so
instead of claiming Enter/Esc behavior.

Centered dialogs retain their type-specific preferred width and height but use
one bounded rectangle calculation: at least one cell remains around the dialog
at every supported size, and neither dimension may exceed the terminal.
Editors may use the largest bounded dialog; compact confirmations may remain
short. Long values ellipsize or wrap within the body. At 80×24 the title,
selected control or operation identity, exact validation/disabled reason, and
cancel route take priority. Below 80×24 the global resize message remains
authoritative and no dialog is rendered.

Selection lists use the semantic table-header and full-row selected styles.
High-contrast and no-color modes preserve the marker, title, controls, and
validation text; reduced motion does not animate dialogs. Opening records the
previous pane, the front typed dialog alone receives input, and close restores
that exact pane unless a typed workflow intentionally transitions to another
dialog. Rendering helpers never mutate dialog state.

---

## 22. Notifications

Non-dialog notifications render in the bounded transient footer slot; they do
not clear or cover the Workspace with a popup. Typed dialogs, including build
completion review, remain modal overlays and use the same footer slot to say
that confirmation is pending. A notification remains actionable/dismissible
through the existing typed `Enter`/dismiss route, and important build failures
retain their action that opens Errors.

The footer projects informational, success, warning, error, reconnecting, and
activity marker-plus-text forms from existing model state. Generic strings are
informational rather than guessed from wording. Exact retained error/warning
logs and typed build completion/cancellation supply result severity. Repeated
backend log lines remain in bounded log retention and do not become one footer
notification per line.

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

The overlay uses three responsive geometries. At 130+ columns it is centered,
up to 112 columns wide, and up to 30 rows high. At 100–129 columns it is inset
three cells on each side and two rows vertically. At 80–99 columns it keeps a
one-cell horizontal and vertical inset and consumes the remaining useful
viewport. It never computes a rectangle larger than the terminal.

Inside the border, regions are stable: shared one-line search, bounded command
table, selected-command detail, then one-line controls. The wide/medium table
columns are `Command | Shortcut | Availability`; narrow rows retain the same
three facts in one bounded cell. Availability is always marker plus text:
`✓ Ready`, `! Limited`, `✕ Unavailable`, `– Unsupported`, `– Unavailable`, or
`? Unknown`; it is never color-only. The selected row uses the shared full-row
selection treatment.

The command-table title reports selected/total result position and visible
window as `Commands · <current>/<total> · rows <start>–<end>`. Selection stays
inside a bounded, centered viewport where practical. The detail region always
names the selected description and `Available: yes/no`; exact disabled reason,
compatibility reason, limitations, and implementation IDs follow in priority
order and ellipsize rather than taking rows from the command list. A no-match
query renders an explicit empty result plus `0/0`; it does not show stale
selected details.

Typing filters case-insensitively across labels, descriptions, and shortcuts.
`Backspace` edits the query, `Ctrl+U` clears it, `Up`/`Down` moves through
filtered results, `Enter` activates an available result, and `Esc` closes the
palette. Empty results show an explicit message. Activating an unavailable
command or an empty result changes no application state.

Palette input is routed before dialog and workspace input and remains
focus-trapped. Opening records the exact active pane. Closing without a
command restores that pane; navigation commands move focus to their selected
workspace, while commands that open dialogs preserve the original pane return
target through the dialog workflow.

---

## 24. Footer and keyboard shortcuts

The footer is always visible in normal layouts. It renders the highest-value
current-context controls that fit, followed by global Help, Menu, and Quit.
The authoritative shared function-key catalog is:

```text
F1 Help  F2 Tasks  F3 History  F4 Dashboard  F5 Logs
F6 Layers  F7 Recipes  F8 Images  F9 Commands  F10 Menu
```

This catalog is shared by input dispatch, Help, and footer rendering. `F9` and
`F10` are intentional aliases for the command palette; the bounded rail omits
the lower-priority `F9` alias instead of advertising it as a nonexistent
global search. There is no function-key terminal route: `F4` truthfully opens
Dashboard, while terminal/session access remains in Navigator, Dashboard, and
the command palette through its actual bindings.

At the canonical `160x48` Tasks size the footer retains its exact two-row
bordered reference geometry. With Navigator focused it prioritizes Navigator
selection/open/prefix controls, then non-current global destinations that fit,
then `F1 Help`, `F10 Menu`, and `q Quit`. With Workspace focused it instead
prioritizes task selection/filter/cancellation and `Tab Inspector`. A route
that already names the active screen is omitted as redundant. Every displayed
key invokes the named action; no unavailable or duplicate route is used merely
to resemble concept art. When transient status is present it takes the bounded
slot before the clock, removing lower-priority optional routes first while
preserving Help, Menu, and Quit.

The clock is fixed at eight digits and right aligned at `100+` columns. It is
hidden at `80..99` columns so current workspace actions and Help/Menu/Quit do
not clip. Items are measured using terminal cell width and are appended only
as complete hints. At constrained widths the rail uses compact highlighted key
tokens; complex SDK, Testing, Security, and QA workspaces retain their existing
compound narrow tokens.

Dashboard example:

```text
B Options  Ctrl+B Prefix  Tab Inspector  ↑/↓ Package progress  F1 Help  F10 Menu  q Quit
```

When no dialog or editor traps input, `q` and `Ctrl+C` retain their global quit
meaning while Navigator, Workspace, or Inspector has focus.

When terminal sessions are available, the footer also shows `Ctrl+B prefix`
and the pending prefix map. Prefix commands are client-local navigation intent;
PTY input remains daemon-authorized by the active writer lease.

Layers example:

```text
Enter Open/Toggle  ← Collapse  → Expand  e Editor  m Metadata  d Dependencies  / Search
```

Tasks example:

```text
↑/↓ Select  f State  F Field  / Edit Filter  d Duration  c Cancel  Tab Inspector
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

The keymap coverage test dispatches the complete shared function-key catalog,
the documented global and focus routes, Tasks and Logs contextual bindings,
all default `Ctrl+B` second keys, and representative modal confirmation keys.
It renders Help and the contextual footers from the same catalog, rejects
duplicate global labels/bindings, and proves unmatched modal input cannot leak
to global routing. Workspace-specific typed input tests remain mandatory for
the specialized bindings documented in their own sections.

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

Rendering uses the public `SemanticTheme` catalog. Every built-in theme and
the no-color override resolves the complete catalog before a widget renders.
Widgets select a role, never a terminal color:

- surface and text: `background`, `primary_foreground`,
  `secondary_foreground`, `heading`, and `table_header`
- structure and interaction: `focused_border`, `inactive_border`,
  `selection_foreground`, and `selection_background`
- lifecycle: `success`, `warning`, `error`, `running`, `pending`, and
  `disabled`
- emphasis: `accent`, `muted`, `informational`, and `progress`
- bounded graphs: `graph_cpu`, `graph_memory`, `graph_disk_read`,
  `graph_disk_write`, `graph_network_rx`, and `graph_network_tx`
- source preview: `syntax_keyword`, `syntax_name`, `syntax_operator`,
  `syntax_value`, and `syntax_comment`

`running` identifies executing work, `pending` identifies queued or waiting
work, and `progress` colors a determinate progress value. A renderer may use
the same resolved color for related roles, but it must request the semantic
role matching the content. `informational` is also the path/link role. Graph
roles belong only to honestly measured bounded series; an unavailable metric
does not receive a decorative graph color.

The persistent shell, workspaces, Inspector, Footer, dialogs, notifications,
tables, gauges, logs, build status, and source preview use these roles. A
theme must provide every role. Adding a role requires updating all built-in
themes and deterministic TestBackend coverage.

Workspace renderers receive only resolved semantic styles. Hardcoded colors
are limited to construction of the built-in theme catalogs and test fixtures;
they are forbidden in production widget rendering.

`monochrome` and `--no-color` use terminal attributes instead of color:

- focused elements are bold
- selections use reverse video
- disabled text is dim
- warnings are bold
- errors are bold and underlined

These modes must not depend on the terminal's default foreground/background
pair to distinguish focus, selection, severity, or progress.

### Accessibility invariants

Accessibility is a property of the rendered terminal buffer, not a separate
workspace or a color-only theme:

- every lifecycle and severity uses a stable marker plus a word (`▶ Running`,
  `✓ Succeeded`, `✕ Failed`, `! Warning`, `? Lost`, and equivalent labels)
- the focused pane has a bold focused border in attribute-only modes, selected
  rows use reverse video, disabled controls are dim and say `Disabled`, and
  errors are bold and underlined
- every determinate bar or gauge includes its numeric percent and context;
  indeterminate work says `progress unknown active` when reduced motion is on
- reduced motion makes the same model state produce the same buffer regardless
  of animation frame; running and pending meaning remains in text
- section titles, table headings, paths, actions, validation errors, and
  shortcut hints remain present as ordinary buffer text so terminal readers do
  not need to infer meaning from box drawing, color, or animation
- responsive hiding may remove lower-priority facts, but never the only focus
  cue, state word, numeric progress equivalent, or disabled reason

The accessibility invariant suite renders representative task, log, history,
health, dialog, and compatibility states in high-contrast, monochrome,
`--no-color`, and reduced-motion modes. It checks semantic text and terminal
attributes independently; passing colored snapshots alone is insufficient.

The resolved Yoctui color preference is authoritative at the terminal backend:
when color is enabled, the backend emits the selected semantic palette even if
the parent process exports `NO_COLOR`; when color is disabled in Settings or by
`--no-color`, widgets emit the attribute-only contract. This prevents the
Settings value and visible result from disagreeing.

### Theme switching

Theme can be changed through:

- Settings workspace
- command palette
- CLI/configuration

The command palette exposes a dedicated `Choose theme` command that opens the
same focus-trapped picker as the Settings Theme row. Choosing or previewing a
named theme enables color unless this launch has the explicit `--no-color`
override. When that override is active, the picker remains usable and names
the override instead of pretending that palette changes are visually active.

In the Settings workspace, activating the Theme row opens a focus-trapped theme
submenu. Up/Down selects a named theme and applies it immediately; Enter keeps
and persists the selection, while Esc restores the theme, color mode, and dirty
state that existed when the picker opened. Theme selection is never a blind
toggle or an implicit cycle.

Theme previews apply immediately; accepted changes persist.

Interactive startup reserves the terminal exclusively for the workbench.
Backend and BitBake standard-error diagnostics must never write directly into
the alternate screen. The backend drains them into a bounded diagnostic tail;
routine startup notes and compatibility warnings remain non-obscuring, while
an actual bridge startup or disconnect failure includes the redacted bounded
tail in the normal typed error/notification path. A live PTY acceptance check
must prove that no pre-frame diagnostics leak into the terminal and that
`Ctrl+P` → `Choose theme` opens the named picker and persists a changed theme.

### Preferences

The Settings workspace is a centered, bounded typed row editor. `Up`/`Down`
(or `j`/`k`) selects a row; `Left`/`Right` or `Enter` changes or opens it. The
schema-v1 rows are:

- theme
- comfortable/compact visual density
- Unicode/ASCII symbols
- animation speed
- reduced motion
- color enablement
- mouse input
- footer shortcuts
- log wrapping
- log following
- pane-size restoration
- automatic/accessible-text charts
- image previews (metadata-only, with the exact transport limitation)
- terminal prefix (fixed `Ctrl+B`, with its reserved-prefix reason)
- keybindings

Changes preview immediately and are atomically saved to `session.toml`.
`config.toml` is a user-authored default and is never rewritten by the TUI.
Session values override configuration defaults for these interactive rows;
hard CLI overrides such as `--no-color` remain authoritative. A failed save
keeps the previewed value, marks Settings as unsaved, and shows a notice.
Pressing `r` retries the atomic save without changing the previewed value.
Pressing `R` restores the validated built-in preference set and a one-pane
layout, then persists it through the same effect. Rows that cannot safely vary
are visibly `locked`; activation is inert except for their exact explanation.

`WorkbenchPreferences` is the sole interactive preference authority in new
session writes. It has an explicit schema version, closed values, unknown-field
rejection, and embedded validated keymap preferences. Legacy top-level session
fields are read once when the schema is absent. The next successful save writes
the schema and normalizes those legacy fields away. Pane topology is restored
only when `remember_pane_sizes` is true. `--no-color` changes only the current
rendering and preserves the schema's stored color choice.

CLI flags are launch-scoped. `--no-color` must not overwrite the stored color
preference on exit, and `--backend` must not become an implicit backend for a
later launch. A durable backend default belongs in `config.toml`; absent a CLI,
environment, or configuration-file choice, startup uses the metadata-capable
bridge. Test and snapshot processes must use private XDG config, state, and
runtime directories and may never rewrite the operator's real session.

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
their typed field names. Popup editors use explicit Normal, Insert, and Visual
modes. `i` enters Insert, `v` enters or leaves Visual, Esc returns from Insert
to Normal, and workflow save/preview actions validate before applying. Existing
destructive confirmation dialogs remain a separate explicit step after
validation; a popup editor never bypasses them.

Every bounded popup editor has a real cursor and value selection. `Home` and
`End` move to the beginning and end of the current line; `h`/`l` and arrow
keys move by character, while `j`/`k` and arrows move by line. `b`/`w` move by
Unicode word class, PageUp/PageDown move by the model-owned viewport height,
`u` undoes, `r` redoes, and `x` deletes at the cursor or selected range. Opening a
single-field edit selects the field value so the first Insert-mode type or
paste replaces it rather than appending after the TOML document. Bracketed
paste inserts at the cursor, and copy copies the selected value or line through
the existing clipboard effect. A persistent final popup row shows the active
shortcuts: `i insert  e change value  Enter save/preview  Esc normal  q close
Home/End line  Ctrl+C copy  Ctrl+V paste`.
The build-target command uses `Build target.toml` with its `target` value and
read-only requested task line; validating it still opens the existing build
confirmation rather than starting a build directly.
SDK publication uses `SDK publish.toml` with its absolute `destination`; its
validated document still opens the existing exact publication confirmation.
SDK native tools use `SDK native.toml` with `mode`, `workspace`, `recipe`,
`tool`, and bounded space-separated `arguments`; validation retains the
existing FindSysroot versus RunNative restrictions before confirmation.
Test launch uses `Test launch.toml`; family, machine, distro, and image are
authoritative context and validation rejects a document that changes any of
them. Scope, selector, parallelism, verbosity, and network policy remain
editable and are validated before the existing launch confirmation. The popup
initially selects `scope` for immediate keyboard replacement and shows typed
validation failures above the persistent shortcut row.

### Persistent terminal-session interaction

Terminal sessions render the daemon-owned emulator screen and bounded
scrollback. Entering a session attaches as a viewer; typing, paste, terminal
mouse reports, and resize are enabled only after the client visibly owns the
single writer lease. Other attached clients remain read-only and show the
writer identity. Taking control is explicit, and loss of the client or SSH
connection releases control without terminating the session.

Each terminal pane keeps its real session/lifecycle title and a compact
viewer/selection line above the daemon-provided screen rows. Screen rows are
typed, dimension-bounded replica data; Ratatui never parses ANSI output. A
session without a retained screen says `Screen unavailable · awaiting daemon
snapshot` rather than presenting metadata as terminal output. The selected
session's screen is rendered only inside its client-local pane; the workbench
header and footer remain outside the PTY viewport.

The configured prefix returns input handling to Yoctui for detach, pane/session
navigation, help, and later split commands; those keys are not forwarded to the
terminal application unless the literal-prefix route is chosen. Detach and
normal client exit leave daemon sessions running. Reattach restores the current
screen, dimensions, lifecycle, and bounded scrollback. Exited and Lost sessions
remain distinguishable, and Lost never implies a surviving process.

Copy/search mode is client-local and operates on bounded daemon-provided screen
and scrollback ranges; copy uses the normal clipboard effect. Paste requires
writer ownership, is bounded, and is sent literally, using bracketed-paste
markers only when the terminal application enabled that mode. Session close or
kill shows the exact session/process-group effect and follows normal destructive
confirmation policy. Keyboard routes remain mandatory; terminal mouse input is
forwarded only while the session is focused, writer-owned, and the application
has requested mouse reporting.

Contextual Open terminal actions are available only when their authoritative
context exists: current build directory, source tree, selected configured
layer, selected authoritative recipe source, current Devtool workspace,
verified SDK environment, and selected image/deploy directory. The preview and
session listing show the typed context identity and resolved directory. The
action never exposes a general shell-command field and never loads commands
from `.yoctui/project.toml`; stale or changed identities disable creation with
an explicit refresh instruction.

The Devtool workspace exposes two terminal-backed interactive routes when
authoritative status is current: Open workspace shell and Edit recipe. Edit
recipe previews the exact `devtool edit-recipe <recipe>` identity before
creating a session; the editor runs in that daemon-owned PTY. Modify,
update-recipe, finish, deploy and reset retain their existing background-job
dialogs and lifecycle rather than opening a terminal merely because PTYs are
available.

Recipe actions expose Devshell and only the interactive configuration tasks
advertised by authoritative BitBake metadata. Kernel menuconfig and U-Boot
menuconfig resolve their current provider identities. The confirmation shows
the exact recipe, task, executable identity, and build directory; acceptance
creates/focuses a daemon terminal session without suspending the Yoctui client.
Unavailable or stale providers/tasks show a refresh reason instead of falling
back to a guessed recipe or free-form command.

Open SDK shell is enabled only for a selected, inspected SDK root containing
exactly one safe `environment-setup-*` file. Its preview shows SDK identity,
setup-file identity, resolved root, and interactive shell. Confirmation first
captures the environment in a bounded child process and then creates a
persistent daemon-owned SDK terminal; it never asks the operator to paste or
export environment expressions. If the setup file changes after preview,
capture fails with a refresh instruction. Open native shell uses the already
verified build environment and build directory and is labeled distinctly from
an installed SDK shell. Neither route reads executable text from the optional
project profile.

The persistent header shows the client replica connection state, daemon-owned
BitBake lifecycle, active daemon job count, daemon PTY count, and a health line
for uptime, queue pressure, resident memory when available, and recovery phase.
These values come only from the current typed daemon snapshot/event stream.
Synchronizing, Current, Stale, and Disconnected remain visually distinct text
states; a disconnect may retain the last values for inspection but must label
them Disconnected. Installing or replacing daemon state never changes the
selected screen, focused pane, Navigator row, theme, open dialog/editor, or
client-local layout. The status UI also exposes the daemon instance identity,
connected client/session counts, recovery warnings, and confirmation-gated
restart/stop actions when those commands are available.

An attached daemon-owned BitBake build installs the same typed build, parse,
runqueue, task, log, warning, error, and terminal state shown by a standalone
build. A fresh attach reconstructs that state from the bounded daemon snapshot;
ordered incremental build events then update it through the normal reducer.
The Dashboard must never remain at `LoadingWorkspace`, `0/?`, or "Waiting for
BitBake task events" merely because the client that submitted the build has
detached. Unknown progress remains unknown, while authoritative task totals and
per-task percentages render using the existing progress meters.

SDK publication and native-tool confirmations submit their exact typed
operation and selected SDK context to the daemon when attached. The client
continues rendering the existing typed SDK session from daemon events; it does
not start a second local runner. If daemon attach is unavailable, the UI shows
the explicit compatibility diagnostic and retains the existing standalone
policy until that policy is selected and tested.

Test result import uses `Test result import.toml` with one normalized absolute
`root`, initially selected for immediate replacement, preserving the existing
bounded typed import operation.
Test comparison uses `Test comparison.toml` with baseline and candidate result
paths; the baseline initially has value selection. Both paths resolve only
against the current typed result inventory before preview. Import and
comparison validation failures render above the shared persistent shortcut
row.

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

When an explicit build directory is available and no daemon can be attached,
the interactive client continues locally with the selected bridge and loads
typed workspace, layer, and recipe metadata. Expected daemon absence is shown
as disconnected status in persistent chrome, not as a centered notice that
obscures the workbench. Metadata failure remains an actionable diagnostic and
must not be disguised as successful empty inventories.

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
The preferred proportions are approximately 16% / 56% / 28%, with a 22-column
minimum Navigator and a 32-column minimum Inspector. Tasks uses the complete
three-tier cockpit when both width and height permit.

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

- hit testing uses the same header, shell, footer, responsive breakpoints, and
  pane proportions as rendering; header/footer padding and the below-minimum
  resize screen are inert
- click focuses Navigator, Workspace, or Inspector by the pane actually under
  the pointer; the narrow pane switcher labels select their named pane
- click selects Navigator rows/tree nodes using the current bounded viewport;
  clicking the selected row follows the same typed activation route as Enter
- wheel input over Navigator follows the Up/Down selection route, while wheel
  input over Workspace follows that workspace's typed Up/Down route; Inspector,
  header, footer, and non-selectable padding remain inert
- modal dialogs trap every mouse event. A click focuses the dialog, and wheel
  input follows the dialog's typed Up/Down choice route only when that dialog
  exposes one; controls without authoritative geometry remain keyboard-driven
  and discoverable in the dialog hints
- click a terminal-session leaf selects the exact session rendered in that
  leaf. Dragging recalculates the focused leaf's nearest resizable parent split
  from its real axis, area, and current ratio; it never infers direction from
  coordinate parity and remains bounded by the model's 10–90% constraint
- mouse Up has no state transition, and clicks or drags outside an actionable
  current-layout region have no state transition
- server-relevant terminal mouse reports are sent only for a focused,
  writer-owned PTY that has requested mouse reporting

Every action must remain fully usable by keyboard.

Acceptance tests resolve mouse input against canonical wide, medium, narrow,
and below-minimum shell geometry. They assert exact Navigator, task, tab, and
PTY-leaf selection; wheel-to-keyboard action parity; modal trapping; inert
non-actionable regions; and axis-correct bounded split resizing.

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

---

## 31. Optional project profiles

A repository may contain `.yoctui/project.toml`. The file is optional: absence
is the normal `No project profile` state and must not disable discovery,
onboarding, builds, or any other Yoctui workflow. Creating or using a profile
requires no changes to Poky, vendor layers, recipes, or BitBake metadata.

The root document has one required `schema_version` and may contain only these
typed team-intent collections:

- recipe, image, and layer favorites identified by portable logical identity
- named build presets containing targets plus optional machine, distro, and
  supported typed build-option preferences
- named workflows containing ordered references to supported Yoctui typed
  actions and presets
- repository-relative references to team-owned files when a typed action
  explicitly supports a file

Schema version `1` is the first supported version. A missing, zero, newer, or
malformed version is shown as `Unsupported` or `Invalid`; Yoctui never guesses
an interpretation or silently rewrites it. Unknown fields are rejected with a
field location so team intent cannot be silently ignored.

Profile paths use `/`-separated repository-relative syntax. Empty paths,
absolute paths, `.` or `..` components, platform prefixes, NUL/control bytes,
and references that resolve outside the canonical repository root are invalid.
Resolution rejects symlink escape. A profile may identify team-owned inputs;
it may not store build-host paths, SDK secrets, credentials, environment
snapshots, shell fragments, or executable names.

Favorites and preset/workflow references are resolved against the current
BitBake-authoritative layer, recipe, image, task, and configuration inventory.
Missing, ambiguous, renamed, or provider-changed identities are displayed as
`Stale` with the original profile identity and an exact reason. They are never
silently substituted. A partially stale profile remains inspectable, but an
action that depends on stale input is disabled.

Loading a profile is read-only and inert. It may populate labels, favorites,
presets, and workflow choices, but it never starts a build, changes
configuration, sources a shell, clones a repository, invokes a tool, or opens
a network connection. Workflows are an allowlisted sequence of typed Yoctui
actions; arbitrary command strings, shell syntax, environment assignments, and
implicit hooks are not part of the schema. Choosing a preset or workflow opens
the normal typed preview and confirmation route for every consequential or
destructive action.

Profile generation occurs only from an explicit `Generate project profile`
action followed by a preview and confirmation. Generation writes a minimal,
deterministically ordered versioned document using portable identities, never
copies personal state, never overwrites an existing file without the normal
replacement confirmation, and never executes generated content.

Theme, animation, layout, recent paths, local aliases, local default preset,
credentials, trust decisions, and other personal preferences remain in the
user-local session/configuration. The project profile cannot override CLI,
environment, user-local safety preferences, current capability state, or
BitBake metadata. The UI labels profile values as team intent and separately
shows the currently resolved authoritative value.

The Build environment workspace includes a `Project profile: team intent`
section. `N`/`n` moves through favorites, presets, and workflows, and `p`
previews or opens the selected resolved item. Each row is labelled `resolved`,
`STALE`, `AMBIGUOUS`, or `UNAVAILABLE` with an exact reason. Recipe and layer
favorites navigate to their existing authoritative selections; image favorites
open Images. A resolved build preset opens the existing typed build
confirmation and does not start a build. A workflow selection remains inert
and requires explicit review of its typed steps. Unresolved entries cannot be
activated. No profile shortcut bypasses existing confirmations.
### QEMU daemon routing

After confirmation, attached clients submit QEMU launch and cancellation
effects to the daemon. The client renders the resulting job state and remains
safe to detach while the emulator continues under daemon ownership.

Confirmed Wic image creation is submitted to the daemon and remains visible in
job history after client detach.

Confirmed selftest sessions are submitted to the daemon when attached; output
and terminal state remain visible while the client detaches or reconnects.

Test-result comparisons submitted while attached use daemon-retained result
generations and report stale-generation failures explicitly.

## Environment-correlated feature availability

The UI is generated from the daemon-owned capability snapshot for the selected
build environment. The installed Yoctui binary knowing a workflow does not make
that workflow available.

Useful unavailable actions remain visible and disabled for discoverability.
Their Inspector/help text states the exact detected reason, for example:

```text
Devtool upgrade
Unavailable
Current Devtool does not expose the upgrade subcommand.
```

Available-with-limitations actions explain the selected fallback or missing
portion before confirmation. Unknown probe results are visibly distinct from
known unsupported behavior. Ordinary workspaces avoid release-number clutter;
a dedicated Environment/Compatibility inspector shows authoritative identity,
snapshot generation, support classification, evidence, missing tools, limited
features, unsupported features, and unknown probes.

Capability updates may arrive while the UI is running. The model revalidates
the current selection and any open dialog against the new generation. A dialog
whose action is no longer safe closes or becomes non-confirmable with the new
reason. Stale generations are ignored, and no capability transition may launch
an invalid command or panic at any terminal size.

When capability loss closes a launch dialog, focus returns to the pane that
opened it and a bounded notification preserves the exact capability reason.
Selections that remain valid do not move. Client-local dialogs and
cancellation of an already-owned process remain usable so a capability update
cannot strand work. If confirmation and a capability update race, the model
rolls back confirmation preparation and emits no effect.

### Environment/Compatibility workspace

`Compatibility` is a first-class destination under the Navigator's environment
and maintenance area and is also searchable from `F10 Menu` / `Ctrl+P`. It is a
client-local view of the current daemon authority: opening it never runs a
probe, command, or version inference. If no current authority exists, the
workspace remains usable and shows `Snapshot: unavailable` plus the exact
daemon/synchronization reason.

The wide workspace contains a compact identity/summary band, a capability
table, and the persistent Inspector. The identity band shows only authoritative
detected values: build directory, source roots, OE-Core/Poky release identity,
BitBake version, DISTRO, MACHINE, layer series, backend/protocol, snapshot
generation, and Full/Degraded/Diagnostic mode. Unknown fields read `unknown`;
they are never reconstructed from neighboring values. The summary always
shows exact Available, Limited, Unavailable, Unknown, and Unsupported counts.

The capability table has `Capability`, `State`, and `Implementation` columns.
Rows are sorted by stable capability ID and use explicit text as well as color.
The default filter is `All`; `1` selects All, `2` Available, `3` Limited, `4`
Unavailable, and `5` Attention (Unknown plus Unsupported). `/` edits a bounded
case-insensitive search over capability ID, reason, requirement, and selected
implementation; `Esc` ends search before performing normal navigation.
`Up`/`Down` or `j`/`k` changes the selected visible row without escaping the
table. Selection remains on the same capability ID across filter, resize, and
newer snapshot generations when that ID is retained; otherwise it moves to the
nearest valid row.

The Inspector shows the selected stable capability ID, state, exact reason and
requirement, every limitation, selected preferred/fallback implementation, and
bounded typed evidence (kind, outcome, subject, detail, and argv when present).
Available rows without a limitation do not invent a reason. Unknown,
Unavailable, and Unsupported remain visually and textually distinct. Long
paths, reasons, requirements, evidence, and argv wrap or truncate within their
panel and never widen the terminal.

Medium layout uses the standard Inspector overlay. Narrow layout uses the
shared visible-pane switcher, keeping identity/summary, table, and Inspector
reachable. The below-`80x24` resize contract remains unchanged. The contextual
footer prioritizes `↑↓ Select  1-5 Filter  / Search  Tab Focus`, then appends
the shared Help/Menu/Quit routes that fit.

### Visible action availability

Every useful environment-backed action remains present in its normal
Navigator, command-palette, workspace, Inspector, dialog, or footer location.
The shared presentation state renders it as Available, Available with
limitations, Unavailable, Unknown, or Unsupported from the centralized
workspace requirement only. Disabled entries cannot be selected as
confirmable operations, but focus/selection may land on them so the exact
reason is discoverable. Limited entries remain usable and explain the selected
fallback or limitation before confirmation. Client-local navigation, settings,
help, copy/open operations, and cancellation of already-owned processes remain
usable without compatibility authority.

Action surfaces show concise state during normal work and place the full exact
reason in the Inspector, command-palette description, or dialog body. They may
name a required capability/tool or maintained alternative, but must not add
release-number policy or generic unexplained `Unsupported` labels. A live
snapshot replacement updates every visible action from one model projection;
widgets never retain a second capability cache.

While an environment-backed dialog is open, a compact two-line compatibility
rail replaces the normal header without reducing or covering the dialog body.
It states Available, Limited, Unavailable, Unknown, or Unsupported, whether
confirmation is available, the exact reason or limitation, and the selected
implementation when one exists. Client-local dialogs do not add this rail.
The rail is presentation only: confirmation is independently revalidated by
the model, and capability loss closes an unsafe dialog with restored focus and
an exact notification.

---

## 32. Raw Mode

`Raw Mode` is a first-class Navigator destination in the `TOOLS` group and a
searchable command-palette destination. It is an expert structured workbench
over BitBake command templates. It is not an arbitrary command launcher, an
embedded shell replacement, or permission to infer support from the bundled
Wrynose 6.0 / BitBake 2.18 reference snapshot.

The tracked reference is
`docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md`. Production
widgets never parse it. A compiled typed catalog retains a stable command ID,
category, exact reference section, reference command template, reference
description, parameter definitions, capability requirements, interaction
mode, and safety class. Catalog validation keeps that traceability reviewable.

### Browser hierarchy and focus

The first level follows the reference Table of Contents, with `Favorites`
pinned before the reference-derived categories. Executable BitBake categories,
reference-only sections, companion-tool sections, and conceptual sections are
visibly distinguished. Reference-only, pipeline, filesystem-debugging, and
conceptual material may provide help or links to an existing typed workspace,
but it cannot appear as a runnable BitBake command. Companion tools remain
owned by their existing typed workspaces unless a later catalog entry supplies
its own exact capability and adapter.

Wide Workspace content uses a bounded category column and a command list; the
persistent Inspector is titled `Inspector: Raw command`. `Left`/`Right` (or
`h`/`l`) changes the active browser column, while `Up`/`Down` (or `k`/`j`)
moves bounded selection in that column. `Enter` on a category activates its
command list. `Enter` on an executable command opens its typed configuration;
on reference-only material it opens only its help. Global `Tab` and
`Shift+Tab` retain the shell Navigator/Workspace/Inspector focus cycle and do
not become an undisclosed Raw subpane focus model.

The exact selected command drives the Inspector immediately. It shows, in
order, description, reference section, template, Available/Limited/
Unavailable/Unknown/Unsupported state, exact reason and selected
implementation, interaction mode, safety class, parameters, and favorite
state. No approximate workspace prose may replace the catalog description.
Disabled commands remain selectable for explanation but cannot open a
confirmable preview.

`/` edits one bounded case-insensitive search across category labels, command
templates, descriptions, and favorite names. Result selection remains an exact
catalog or favorite identity. `Ctrl+U` clears it and `Esc` finishes editing
without clearing. `f` adds or removes the selected command/defaults as a
favorite through a typed confirmation when removal or replacement is involved.
`H` opens bounded Raw history. `Esc` moves outward from command list to
category browser, or through the global pane behavior when already at the
outer level.

### Command configuration and parameters

Every executable selection opens a focus-trapping `Run BitBake Command` typed
form. It names the immutable command template and exposes only its declared
parameters plus one bounded `Additional arguments` value. Parameter kinds are:

- Recipe: authoritative recipe inventory picker plus validated manual entry
- Image: authoritative image inventory picker plus validated manual entry
- Target: authoritative recent/current targets plus validated manual entry
- Task: exact selected-recipe task inventory plus validated manual entry where
  the template permits an arbitrary BitBake task
- UI: an enumerated choice from positively discovered implementations
- File: a normalized path editor with the catalog-declared read/write and
  containment policy
- Value: bounded text with a parameter-specific character policy
- Number: a bounded integer with catalog-declared minimum and maximum
- Multiconfig: authoritative configured identities plus validated manual entry

Inventory absence never converts a required parameter into an empty valid
value. Manual recipe/image/target/task/config values are single native argv
elements; empty values, leading option ambiguity where disallowed, control
bytes, and parameter-specific invalid syntax remain inline validation errors.
Selector and manual editing coexist: choosing an inventory value fills the
same typed field and editing it does not create another authority.

The shared popup-editor convention owns cursor, selection, clipboard, and
Normal/Insert behavior. `Enter` validates the current document; a valid form
opens a separate preview and does not execute. `Esc`/`q` closes without work.
The form remains usable at `80x24`, preserving title, selected field,
validation, preview action, and close hint.

The form is a modal focus target. In Normal mode `Tab`/`Down`/`j` and
`Shift+Tab`/`Up`/`k` move between declared fields and the final Additional
arguments field. `Left`/`Right` chooses the previous/next authoritative
inventory value when the selected parameter has a selector; otherwise it moves
the single-line editor cursor. `i` enters Insert mode and `e` selects the whole
current value and enters Insert mode. Insert mode accepts ordinary text and
backspace, retains cursor/clipboard navigation, and `Esc` returns to Normal.
In Normal mode `Esc` or `q` closes without execution. `Enter` from either mode
validates all fields and additional arguments and, only when valid, opens the
separate exact preview. Unmapped shell, pane, and global keys are consumed
while the form or preview is modal.

### Expert arguments and exact preview

Additional arguments are tokenized into native argv without shell evaluation.
Single and double quotes may group characters and are removed by the tokenizer;
backslash may escape only the next ordinary character under the documented
tokenizer grammar. The editor retains at most 12,288 input bytes and validation
accepts at most 64 arguments, 512 bytes per argument, and 8,192 aggregate
argument bytes. A backslash cannot make a control character or forbidden
operator character safe, and the reconstructed argument is checked again.
Unterminated quotes, NUL/control bytes, an empty option name, excess argument
count/length/aggregate bytes, and the shell operators `|`,
`>`, `>>`, `<`, `&&`, `||`, `;`, `$()`, and backticks are rejected. Operator
characters inside a literal parameter are accepted only when that parameter's
typed policy explicitly permits them and they still remain one argv element;
they never gain shell meaning.

The preview always shows the exact executable identity and every argument as
an indexed row. It also shows command ID/catalog revision, connected capability
generation, build directory, interaction mode, safety class, and any
limitations. It never renders a command string as execution authority.
`Enter` confirms only when the exact current capability and inputs still agree;
`Esc` returns without execution.

Safety is a closed catalog value: `Read only`, `Build/mutating`, `Destructive`,
`Server lifecycle`, `Interactive`, or `Unsupported reference`. Force, stamp
invalidation, cleaning, configuration injection, output-file replacement, and
server lifecycle are never silently treated as ordinary read-only work.
Destructive and server-lifecycle commands require the existing separate exact
preview and explicit confirmation strength. Credential-bearing remote-server
tokens are unsupported until a secret-safe input and redaction contract exists;
they are never stored in a favorite or history record.

### Execution, detach, and history

The catalog declares each executable command `Noninteractive` or
`InteractivePty`. A noninteractive confirmation creates a daemon-owned Raw
background job using exact native argv. Its execution view shows command
identity, state, elapsed time, stdout/stderr origin, bounded output/drop counts,
exit code/result, follow/pause, search, vertical/horizontal scroll, save through
an explicit typed destination, cancellation, detach, and reattach. Closing or
detaching the view never implicitly cancels the job.

The execution workspace uses `Up`/`Down` (or `k`/`j`) for bounded vertical
scrolling and `Left`/`Right` (or `h`/`l`) for bounded horizontal scrolling.
`f` toggles follow/pause, `/` edits the bounded client-local search,
`Ctrl+U` clears that search, and `1`/`2` selects stdout/stderr when the narrow
layout can show only one stream. `c` requests typed cancellation, `d` detaches
the current client, `r` reattaches it, and `Esc` returns, detaching an attached
nonterminal execution without cancellation.
Unavailable, terminal, already-attached, and already-detached actions remain
visible but reject with an exact reason; keys never become terminal input for
an interactive PTY. PTY input continues to use the established writer-owned
terminal-session route and prefix layer.

`Enter` in the exact noninteractive preview is the sole start gesture. It
revalidates the current catalog and capability projection, creates a fresh
opaque request identity, and submits only the typed confirmed request to the
attached daemon. A missing daemon, stale generation, changed preview, duplicate
request, or rejected executable/build identity leaves no local process and
reports the exact rejection. Reconnect installs the daemon's current bounded
replica; if daemon process ownership itself was lost, the replica is detached
and terminal `Lost`, never shown as resumed work.

An interactive confirmation creates a daemon-owned PTY session through the
existing terminal/session architecture. The session retains exact command and
workspace identity, emulator screen, writer lease, resize, detach/reattach,
termination, and final status. Raw Mode does not line-buffer an ncurses UI or
parse its terminal output. Detaching returns to Raw Mode and leaves the session
alive; termination remains its existing separately confirmed action.

Raw history is bounded and ordered newest first. It retains command/template
identity, sanitized parameter/default values, start/end time, interaction mode,
terminal outcome, and durable job/session reference where safe. PID, process
group, writer lease, secret, complete output, and temporary path authority are
not persisted. A retained record reopens current compatibility inspection and
configuration; it cannot replay work without a fresh preview and confirmation.
Free-form text and file parameter values are omitted rather than risking secret
or temporary-path retention.
Removed catalog commands remain explicit stale history records: activating one
reports that it is unavailable and does not silently discard it, open a form,
or emit an execution request.

### Favorites

A favorite stores a stable command-template ID, user-visible bounded name,
validated parameter defaults, validated additional argv, and ordering. It does
not store PID, output, job ID, capability authority, or transient process
state. Favorites are atomically persisted in user-local session state and are
never written into `.yoctui/project.toml`.

Favorites remain visible when their command is unavailable or the catalog has
changed. They say `STALE` or the exact five-state availability reason and
cannot run until revalidated. Add, remove, rename, reorder, edit defaults,
inspect, and execute all use typed reducer actions. Execution always rebuilds
the form and preview against the current catalog and daemon snapshot.
Template identity covers executable argv shape, parameter schema, capability
requirement, interaction, and safety class. Opening a current favorite creates
a fresh form with its defaults and additional argv; it does not create a
preview or request until the ordinary review flow is completed.

The Favorites workspace is a bounded ordered record view. `↑`/`↓` select,
`Enter` reopens a current record, `i` inspects its compatibility reason,
`[`/`]` reorder, and `x` opens the exact-name removal confirmation. The view
shows the name, reconstructed command template, defaults, additional argv,
stale marker, and current availability state at narrow and wide widths.
Mouse clicks use the same row geometry as keyboard selection; wheel events move
the active category, command, history, or favorite selection and never bypass
dialog or PTY focus traps.

### Capability and responsive behavior

Every command has an explicit centralized capability requirement. The current
daemon `CapabilitySnapshot` is the only runtime authority. Positive direct
option/task/UI evidence is preferred; conservative catalog-declared fallback
may yield Limited; missing, negative, conflicting, stale, disconnected, or
unknown evidence fails closed. A newer snapshot reprojects list/Inspector
state immediately. It closes an unsafe open form/preview, restores the prior
pane, records the exact reason, and emits no start effect. Already-owned cancel,
detach, reattach, and inspect actions remain available.

At `130+` columns the Workspace shows categories and commands beside the
persistent Inspector. At `100..129`, categories and commands share Workspace
and the global Inspector overlay supplies help. At `80..99`, the existing
Navigator/Workspace/Inspector switcher applies; Raw Workspace shows one of
category, command, form/output, or history state at a time with explicit
back/forward text. Below `80x24`, only the global resize screen renders.
Selection and search identities survive resize.

Mouse clicks and wheel movement use the same rendered rectangles and typed
actions as keyboard selection; modal and PTY ownership rules remain unchanged.
Every capability, safety, favorite, focus, job, stream, and terminal state has
a text marker/word. No-color uses attributes, high contrast preserves exact
meaning, and reduced motion changes no execution or selection state.

The full Raw footer is:

```text
←/→ Pane  ↑/↓ Select  Enter Open  / Search  f Favorite  H History  Tab Focus  F1 Help  F10 Menu  q Quit
```

The execution footer prioritizes `f Follow`, `/ Search`, `c Cancel`, `d
Detach`, `r Reattach`, and `Esc Back` before optional hints. Narrow labels may
abbreviate words but may not hide cancel/detach or misstate availability.

## 33. One-stop workbench usability contract

M21 refines the existing workbench without replacing its persistent shell,
typed event boundary, focus traps, safety rules, responsive breakpoints, or
function-key destinations. The complete researched delivery plan is
[`workbench-ux-roadmap.md`](workbench-ux-roadmap.md); this section records the
authoritative interaction rules that each implementation task must preserve.

### Actions menus and bindings

One model-owned action catalog is the sole authority for action IDs, labels,
descriptions, scope, menu path, shortcuts, aliases, requirements, safety,
palette search, footer priority, Help grouping, availability, and disabled
reason. Application menus, context menus, command palette, Help, footer, mouse
routes, keybinding settings, and keymap tests are projections of that catalog.
They cannot define independent actions or bypass typed confirmation.

The implemented catalog currently contains 27 global commands and 110
contextual workspace operations, 137 definitions in total. Every entry has a
validated lowercase stable ID, typed scope and target, complete
presentation/search metadata, explicit
local and environment requirements, safety class, footer priority, and Help
group. The command palette, contextual compatibility action presentations, and
the catalog section of Help consume those definitions directly. The catalog
corrects the former false `F5` image-build hint to the real `B` route; `F5`
remains Logs. The keymap and menu implementations extend configuration and
presentation from these IDs without creating a second action inventory.

`F10` opens a focus-trapped Workspace/Build/Navigate/View/Tools/Help menu.
Arrow keys move, `Enter` opens/activates, `Esc` moves outward, and bounded typed
prefix selection may select by label. The selected-item action route is `a` or
right click. Disabled entries remain visible and explain the exact missing
selection, authority, capability, or safety prerequisite.

The implemented application menu keeps those six groups in a fixed order and
projects its rows from the global command catalog. The contextual menu projects
the active workspace destination's catalog actions; it never invents an
operation for a selected row. Both overlays retain one bounded selection and a
32-character type-ahead prefix, trap unmatched input, render their selected
stable action ID, shortcut, safety class, and exact disabled reason, and close
outward with `Esc` or `F10`. Activation closes the menu before invoking the
existing typed command action or workspace input route, so compatibility
revalidation and normal/destructive confirmations remain unchanged. A right
click is decoded separately from left-click focus and opens the same contextual
projection. Reduced motion and no-color use the same textual markers and safety
labels, and the overlay remains bounded at wide, medium, and `80x24` layouts.

The shared F1–F10 destinations in section 24 remain unchanged. Collection
navigation consistently supports arrows and `j`/`k`, `PageUp`/`PageDown`,
`Home`/`End`, and `gg`/`G`; trees add `h`/`l` collapse/expand behavior. `/`
searches, `Ctrl+U` clears, `n`/`N` moves through matches, `Enter` is the primary
open action, `Space` toggles checkable state, `a` opens actions, and `?` opens
contextual Help. A focused popup editor or terminal owns its documented input
instead of these workspace routes.

The implemented collection route normalizes real terminal PageUp/PageDown,
Home/End, `gg`/`G`, arrows, `j`/`k`, and Workspace mouse-wheel input before
emitting the existing typed selection actions. A page is ten rows until a
workspace supplies a measured viewport; every reducer still clamps against its
current filtered authoritative inventory. `gg`/Home and `G`/End are catalog
commands, so menus, palette, Help, effective bindings, and keyboard activation
share one route. The former single-`g` conflicts are now `A` for selected-recipe
dependency analysis and `i` for layer-browser Git Inspector context.

`BoundedScroll` is the pure selection/offset/viewport/total contract. Resize,
empty or replaced inventories, and extreme deltas reconcile in constant time,
keep selection visible, and expose a textual current range. Log filtering and
retention eviction first reconcile the selected retained log ID; follow pins
the retained tail, while any vertical move pauses follow at an exact retained
position. Raw output vertical and horizontal positions clamp to retained line
and character bounds rather than a synthetic maximum.

Custom bindings are keyed by stable action ID and scoped context. Loading or
editing rejects active same-scope collisions, reserved terminal-prefix
conflicts, invalid sequences, and removal of the last reachable route to a
critical action. Reset and effective-keymap export are mandatory.

The implemented schema is `yoctui` keymap version 1. A sequence contains one
to three closed typed strokes; equality and prefix ambiguity are both
collisions. Overrides replace all defaults for one catalog action in its exact
binding scope, while omitted actions retain every catalog default and alias.
Workspace bindings take precedence over global bindings, so `i` can select an
image in Images without changing the global Open Images route. `x e` is a real
bounded Configuration chord. `Ctrl+B` cannot begin a global or Terminal
Sessions binding because the PTY prefix remains authoritative. Help and
Dashboard cannot lose their last configured route.

The general app input boundary resolves this effective keymap only after
dialogs, the palette, editors/search fields, and terminal ownership have had
their input. Existing specialized workspace routes that are not command-target
catalog entries remain on their typed handlers until their dependent M21 menu,
focus, and scrolling tasks project them into the same model. A rejected or
overridden catalog default cannot leak back through the legacy global router.
Keymap preference rendering and capture controls belong to the next task.

Settings now includes a Keybindings row that opens a focus-trapped effective
keymap overlay. The table is searchable by action ID, label, menu path, scope,
or binding and always names exact Global/workspace scope, effective sequence,
and `default`, `custom`, or `disabled` state; critical routes are labeled in
text. `Enter`/`c` captures up to three strokes, `Ctrl+S` validates and saves,
`Backspace` edits, and `Esc` cancels capture. `x` removes the selected binding,
`r` resets it, `R` resets all, `e` exports the bounded deterministic report,
and `p` retries a failed session save. Invalid capture remains pending with the
model's exact collision, reserved-prefix, scope, or reachability reason. Only a
validated candidate replaces the live keymap, and persistence failure leaves
the changed in-memory preference visibly dirty and retryable.

### Focus zoom and scrolling

Exactly one pane, subview, menu, dialog, palette, or terminal owns input. Focus
has a textual marker as well as semantic styling. Typed subfocus may select a
workspace section; `Esc` moves outward predictably, `Tab` never enters a hidden
or disabled target, and modal or terminal ownership still traps input.

Zoom temporarily assigns the body to the focused work area while retaining the
header, a compact location breadcrumb, and footer. Closing zoom restores exact
focus, subfocus, selection, vertical/horizontal offsets, follow state, and
layout. Every bounded scroll view uses the same row/page/top/bottom actions,
mouse equivalents, and textual retained-position indicator. Resize, filtering,
search jumps, follow/pause, and retention eviction clamp without losing stable
selected identity when it remains available.

The implemented focus model retains `WorkspaceSubfocus` (`Main`, `Secondary`,
`Context`), `InspectorSubfocus` (`Facts`, `Output`, `Actions`), and an optional
pane-only zoom target. Logical Workspace subfocus is clamped to the sections
supported by the active screen; Navigator has the single textual `Tree`
subfocus. The footer names `Pane/Subfocus`, while zoom replaces the body with
only that pane beneath a one-line `ZOOM · Screen · Pane/Subfocus · Esc restore`
breadcrumb. Header and footer remain present. Toggling zoom never copies or
resets workspace selection, vertical/horizontal offsets, log follow state, or
terminal replicas. `Esc` first leaves a modal owner through that owner's route,
then restores zoom, then resets non-primary subfocus, then follows the existing
pane-outward behavior.

Six read-only global catalog commands—focus Navigator, Workspace, or Inspector;
previous/next subfocus; and toggle pane zoom—make this model directly reachable
from both the F10 View group and command palette. They have no hidden single-key
fallback and therefore do not steal workspace shortcuts. Direct pane focus
while zoomed changes the zoom target, responsive resize retains the same typed
target, a zoomed non-terminal pane owns its full mouse body, and menu, dialog,
palette, and terminal input traps remain authoritative.

### Widgets progress logs and editors

Shared render-only primitives own styling and responsive projection for gauges,
meters, sparklines, charts, bar charts, tabs, scrollbars, legends, checkboxes,
trees, and textual fallbacks. Widgets receive typed values and presentation
state; they do not sample the host, scan files, parse process output, or become
a second interaction authority.

The shared visual projection vocabulary is now concrete. Fractions retain the
exact numerator and denominator and derive an overflow-safe bounded whole
percent; a zero total is `unknown`, and a reported numerator beyond its total
is `partial` while retaining both reported numbers. Histories retain only their
configured newest points; bars, tabs, and legends retain configured bounded
prefixes; scrollbar geometry is reconciled by the model-owned bounded scroll
contract. Available, active, empty, unknown, unavailable, partial, successful,
failed, and cancelled terminal states carry stable marker-plus-word text.

Ratatui gauges, line meters, history charts, bar charts, tabs, legends, and
scrollbars are render-only adapters over those projections. Every determinate
gauge or meter includes `current/total (percent%)`; histories include a current
text value or exact state, bars include numeric values, selected tabs retain
brackets, legends pair label and value, and scrollbars include a numeric range.
ASCII uses `=`, `-`, textual chart levels, `#`, `|`, and ASCII state markers.
Reduced motion replaces changing activity punctuation with stable `active`;
no-color uses attributes supplied by the complete semantic theme catalog.
When space cannot retain both state and detail, the complete state word wins.

Determinate build, parse/runqueue, task, job, resource, and sstate progress
always includes exact numeric or numerator/denominator text. Unknown progress
uses an indeterminate text/activity state and never a zero-percent gauge.
Reduced motion replaces animation with stable lifecycle text.

Indeterminate activity uses the reviewed four-phase `BLACK_CIRCLE` symbol set
in Unicode terminals and the four-phase `|/-\\` set in ASCII presentation. The
phase is a pure projection of the reducer-owned animation tick and configured
fast/slow divisor; the widget never advances itself and random stepping is not
compiled. Every indicator remains adjacent to `loading`, `running`, `waiting`,
or `progress unknown`. Reduced motion emits only that stable lifecycle text.
Success, failure, and cancellation have static markers and never retain an
active throbber.

These scopes are independent members of one typed progress hierarchy: overall
build, parse, runqueue, selected task, selected background job, CPU, RAM, build
filesystem, and sstate reuse. A scope without authority stays unavailable even
when a neighboring scope is determinate. Current-without-total is `current/?`
and active, an ended phase below its reported total is partial, and terminal
build/task/job projections retain their last authoritative fraction. Average
task rate and ETA are projections from completed work plus injected elapsed
time and always begin with `estimate`; no selected entity or invalid resource
sample remains explicit. Because no typed backend field currently reports
sstate reuse progress, that scope says unavailable rather than inferring logs.

The build Logs workspace retains its typed bounded store. A separate internal
diagnostic view may display Yoctui tracing records, but it never captures or
reclassifies BitBake domain logs. Both expose exact source, filtering, search,
scroll/follow state, retention/loss accounting, and bounded copy/export.

The reusable editor is reducer-owned and bounded. It owns multiline Unicode
input, byte-boundary-safe cursor/selection, 64-entry undo and redo histories,
word/line/page motion, bounded search/replace, explicit Normal/Insert/Visual
modes, line-number and wrap metadata, and exact validation ranges. Clipboard
and bracketed paste are distinguished typed sources and are rejected above the
payload bound without partial mutation.

Clean, modified, diff-preview, external-conflict, saving, saved, and failed
save states are distinct. Atomic-save requests include the content revision and
a same-directory temporary path; recoverable failures retain retry state.
Rendering and filesystem execution remain separate adapters. The persistent
mode line names save/preview/cancel/copy/paste/undo/redo controls. A third-party
textarea renderer cannot place Ratatui state in `yoctui-model`.
The evaluated `ratatui-textarea` adapter is rejected: Yoctui's stateless custom
renderer projects the complete model without duplicating widget-owned editing
state or adding the candidate dependency closure.

Checkboxes distinguish checked, unchecked, indeterminate, disabled, and focused
states with text or ASCII equivalents. `Space` changes selection only; a batch
operation requires its ordinary action and confirmation sequence.

### Rootfs composition

Images gains an image-correlated Rootfs composition subview. Installed-package
composition comes from the exact image manifest plus authoritative bounded
pkgdata. Filesystem composition is optional and comes only from the exact
BitBake-reported `IMAGE_ROOTFS` for the selected image/build identity.

The adapter requires canonical build containment, never follows symlinks,
deduplicates hard links, identifies special files, and enforces entry, depth,
byte, time, and cancellation bounds. Missing or cleaned work state is
Unavailable; hitting a bound is Partial with the exact limitation.

Wide layouts may show a pie chart, exact-byte/percentage table, legend, and
drill-down tree. Medium layouts prefer bars plus table. Narrow, ASCII,
no-color, and terminal-reader-oriented layouts use table/tree text. Small
categories combine only into an explicit inspectable `Other` group. Package
and filesystem authorities never share a total or silently substitute for one
another.

Terminal-native artifact previews are not part of this contract. The evaluated
deploy inventory has no raster MIME authority: rootfs and Wic records are
storage images, kernel/bootloader records are binaries, and the remaining
records are metadata or unknown. The Images Inspector therefore skips graphics
probing and exposes exact metadata or Rootfs composition as a textual fallback
on direct terminals, SSH, tmux, no-color, reader-oriented modes, and
TestBackend. A future preview requires a new typed raster authority and a fresh
bounded transport/decode/dependency review.

### First-class terminal sessions

The daemon remains the sole owner of PTY processes, `vt100` emulation, bounded
screen/scrollback, attachments, writer epochs, input, resize, and termination.
The client UI consumes typed terminal replicas and never reparses ANSI.
`tui-term` 0.3.4 is admitted only through its generic `Screen`/`Cell` traits
with all features disabled. Ordered sparse protocol cells are bounded and
validated, expand into one client grid, and drive both styled rendering and
derived plain-text fallback. The transient adapter preserves Unicode width,
colors/modifiers, cursor visibility/position, scrollback coordinates, viewport
offsets, and no-color meaning; it owns no parser, PTY, process, input, resize,
or retained screen state.

Terminal Sessions is a normal discoverable destination with context-aware
creation, session list/tabs, splits, zoom, rename, writer/read-only and
take-control state, copy/search, paste, dropped-history accounting,
detach/reattach, close, and separately confirmed process-group termination.
All terminal input except the configured prefix goes to the writer-owned PTY.
Prefix Help exposes session/pane navigation, copy/search, detach, and literal
prefix. Disconnect, daemon restart, terminal exit, and process loss remain
distinct outcomes.

### Dependency and accessibility gate

Every third-party widget requires a refreshed license/MSRV/source/checksum,
feature, transitive-dependency, Ratatui compatibility, notice, SBOM, locked
build, and `cargo deny` review. Showcase applications contribute interaction
research only; their code, screenshots, themes, and assets are not copied.

All new views pass wide, medium, narrow, and below-minimum rendering; keyboard
and mouse parity; no-color, ASCII, high contrast, and reduced motion; real PTY
input; bounded large-data behavior; and the existing 10 ms/frame performance
ceiling. The 2026-08-27 live M21 run also passed menus/availability, progress,
completion, safe failure, image manifest/pkgdata/rootfs state, context-terminal,
and reconnect scenarios during an exact Poky 5.2.4 `core-image-minimal` build.
The [keymap reference](keymap.md), [Rootfs evidence guide](rootfs-composition.md),
and [compatibility record](compatibility.md#current-one-stop-workbench-live-evidence)
are the operator-facing boundaries for those results.

## 34. Concept-to-live parity contract

The six reviewed M21 concept images remain non-authoritative visual direction.
M22 acceptance is scenario-based: each manifest entry declares concrete
features and separately identifies production TestBackend, deterministic raster,
and live PTY evidence. Text anchors alone cannot establish that a workflow was
composed, focused, navigated, or operated as shown.

Failed-build acceptance requires the Errors workspace to compose an explicit
failed summary, structured error/warning inventory, correlated paused log with
loss and match position, textual filter selection, and recovery actions. Rootfs
acceptance requires chart and exact composition table to coexist at the
canonical width, with accessible checkbox semantics and a separately labelled
filesystem tree. Editor acceptance requires the production recipe editor and
the focus-trapped F10 application menu to be visible in the same scene with the
menu owning input. Terminal acceptance requires the live client to navigate to
Terminal Sessions, create or attach daemon-owned sessions, render split panes,
show writer/read-only ownership, and expose prefix help.

Raster validation must be derived deterministically from exact production
cells/styles with a pinned renderer and font. It is review evidence, not a new
UI authority. Live evidence must record the exact input sequence and assert the
resulting screen anchors before a scenario can be named as passed. Unsupported
host combinations may produce diagnostics but cannot replace supported-host
evidence.

Every production concept fixture must keep `navigator_selection` aligned with
the workspace it renders. The selected Navigator row cannot retain a prior
workspace identity beneath Dashboard, Errors, Images, Recipes, or Terminal
Sessions. The editor scene additionally anchors the selected file's language,
mode, modified state, and cursor position so a simpler legacy editor cannot
satisfy the concept contract.

The M22 raster implementation uses fixed `160x50` input, `10x20` pixel cells,
gray antialiasing, full hinting, and pinned Cairo/PyCairo plus regular/bold
DejaVu Sans Mono hashes. The provenance manifest records each source-cell and
output SHA-256. Raster check mode must regenerate and byte-compare all six
PNGs; a PNG with no matching exact production-cell source is invalid.

Supported live acceptance uses one checksummed binary for all six workflows.
Each scenario retains its exact PTY stream, a `160x50` symbol/style cell screen,
a semantic text screen and metadata record, explicit interactions, observed
assertions, and a `1600x1000` cell/style raster. ANSI SGR foreground,
background, and bold state must survive PTY composition; a monochrome rendering
of semantic text is not visual evidence. The aggregate manifest must match the base live-build identity
for source commit, binary, Ubuntu 24.04/glibc 2.39 host, official Poky 5.2.4
revision, BitBake 2.12.1, machine, target, and run timestamps. Every artifact is
checksummed; missing attribution, unsupported hosts, stale hashes, open gaps,
or a different binary fail parity.

The six supported-host live rasters are also retained as operator-visible
historical capture evidence under `docs/design/m22/live-scenarios`. The gallery
must contain exactly one ordered screen for each M22 scenario, preserve the
captured source commit, binary, host, Poky, BitBake, machine, target, terminal
geometry, and raster geometry, and remain byte-identical to the attributed live
evidence. Its README image list and machine-readable manifest are part of the
evidence-integrity contract. Fixture- or production-cell rendering cannot
replace fresh live acceptance; changing the historical capture requires new
supported-host evidence.

## 36. M21 visual resemblance contract

Visual parity is separate from workflow-anchor parity. At the canonical
`160x50` geometry, Dashboard, Tasks, Errors, rootfs composition, recipe editor
with F10 menu, and terminal sessions use the M21 workbench silhouette: a
two-level status header, persistent contextual footer, scene-appropriate
navigator width, dominant center workspace, bounded right inspector, cyan
section titles, semantic state colors, and full-row selection.

Dashboard retains Build Overview, Recent Builds, Quick Actions, telemetry, and
Project Inspector as distinct regions. Recipe editing retains the Navigator,
recipe-file tree, syntax-aware document, Recipe Inspector, validation/diff rail,
and an anchored application dropdown in one composition. A test that checks
only words or actions cannot close this visual contract. Deterministic
cell/style rasters are reviewed first, followed by six fresh styled PTY captures
from the same release binary.

## 37. Operator shell polish contract

The Dashboard telemetry strip uses an original history-first terminal-monitor
vocabulary. CPU, RAM, and Build FS render a bounded 60-sample trend field, a
one-row threshold meter, and exact percentage/context. Active meter segments
cross semantic green, warning, and error thresholds; the remainder stays
visibly muted. This design may draw general inspiration from terminal resource
monitors, but it must not copy btop source, strings, or screen geometry. Narrow
network and disk cells stack the current rate over retained history so a
responsive layout cannot silently hide the graph.

Left/Right cycle Navigator, Workspace, and Inspector in both directions; Tab and
Shift+Tab remain aliases. Navigator h/l collapse and expand groups. An Unknown
destination reserves marker width without printing `?`; unavailable, limited,
and unsupported states retain their distinct markers and explanations. Compact
task lists are left-aligned. Active rows begin with a compact circular throbber,
falling back to `●` under reduced motion and `*` in ASCII mode; inactive rows
retain the same indentation and unavailable totals use an em dash.

The persistent header gives Project, Target, Machine, and Distro values the
informational highlight role. Daemon status includes `Local`, `SSH <client-ip>`,
or `SSH (IP unavailable)`; the latter is required when SSH is detected but its
first endpoint is not a valid IPv4/IPv6 address. Theme selectors use color names
such as `Dark gray` and `Light gray`; the serialized enum names remain unchanged
for preference compatibility and never appear in the UI.

`q` and `Ctrl-C` always open “Are you sure you want to exit yoctui?”. `c` while
a build is Parsing or Running opens “Are you sure you want to cancel the
build?”. `y`, `Y`, or Enter confirms; `n`, `N`, or Escape declines. Declining is
side-effect free. Confirmed build cancellation still passes through current
capability authority immediately before any backend effect is emitted.

## 35. Integrated Devtool editing and shell workflow

The Recipes workspace is the owning surface for one continuous development
loop:

```text
refresh status → devtool modify → edit source → save → build recipe
               → devtool update-recipe or finish into a configured layer
```

The exact selected recipe identity and provider path remain authoritative
through every transition. A successful `modify` refresh opens only the
reported absolute Devtool workspace. `Ctrl+B` in that editor builds the owning
recipe, never an image. `update-recipe` updates the recipe selected by that
identity; `finish` retains its existing clean-commit and configured-layer
picker requirements. Job output, cancellation, navigation retention, and
failure recovery remain the existing persistent typed Devtool behavior.

### Source and recipe editor

Recipe metadata and Devtool source trees use the same two-pane editor shell:
bounded file tree on the left and reducer-owned document on the right. The
document uses the shared `TextAreaState` contract: Normal/Insert/Visual modes,
Unicode-safe cursor and selection, line/word/page/document motion, line
numbers, bounded search and match navigation, copy/paste, undo/redo, local diff
preview, explicit clean/modified/saving/saved/failed/conflict state, and atomic
save requests. Dirty content must be saved or discarded explicitly before
changing files, closing, building, updating, or finishing.

The selected path determines a closed language identity. The first supported
set is BitBake, C, C++, Rust, Python, shell, JavaScript/TypeScript, JSON, TOML,
YAML, Make, Markdown, and plain text. The title/status line names that identity.
Syntax presentation uses bounded lexical classification for comments,
keywords, names, operators, strings, and values; it does not claim compiler or
LSP results. Local diagnostics are explicitly labelled structural and include
only deterministic checks owned by Yoctui. External compiler/BitBake failures
continue to arrive through typed build jobs and Logs/Errors.

The editor footer prioritizes `i` insert, `Esc` normal/outward, movement,
`Ctrl+S` save, `/` search, `n/N` matches, `u` undo, `Ctrl+R` redo, `v` visual,
`Ctrl+B` build recipe, and the Devtool update/finish routes. Responsive layouts
may shorten labels but must preserve mode, dirty/save state, language, cursor
line/column, and a reachable build/publish route.

### Interactive-session destination chooser

Every request to start a build shell, selected-recipe devshell/menuconfig, a
Devtool workspace shell, or interactive `devtool edit-recipe` first opens one
focus-trapped chooser. It shows the exact session kind, recipe when applicable,
working directory, executable/argv summary, and two destinations:

1. `Embedded in Yoctui` (default): create the existing daemon-owned PTY and
   navigate/attach to Terminal Sessions.
2. `Detached terminal`: start a supported local desktop terminal emulator with
   the same validated native argv, working directory, and initialized build
   environment. The terminal lives independently after launch.

`Up`/`Down` changes destination, `Enter` confirms, and `Esc` cancels. Neither
opening nor cancelling the chooser spawns a process. Detached launch is
disabled with an exact reason when no supported emulator is available or the
client has no graphical session; Yoctui never constructs a shell command.
Menuconfig and devshell remain interactive PTY operations rather than ordinary
captured BitBake jobs. Noninteractive Devtool modify/update/finish/deploy/reset
remain persistent background jobs.
