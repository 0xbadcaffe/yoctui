# Yoctui Architecture

## Purpose

Yoctui is a Rust/Ratatui terminal workbench for Yocto and BitBake. BitBake remains the authority for metadata and build state. Yoctui requests operations, normalizes events, stores bounded state, and renders typed views.

## Architectural principles

1. Domain state is independent of terminal rendering.
2. UI consumes typed state and emits typed actions.
3. Raw backend output is normalized before reaching widgets.
4. Long-running work is represented as persistent background jobs.
5. Destructive actions are previewed and confirmed.
6. External tools are adapters behind shared execution contracts.
7. Bounded memory behavior is mandatory.
8. Live compatibility claims require live validation.

## Component responsibilities

### `yoctui-model`

Owns:

- domain state
- task, build, job, dialog, notification, and workspace models
- typed actions
- pure reducer
- bounded log and history retention
- selection, focus, and navigation state

Must not:

- spawn processes
- access the terminal
- parse raw BitBake text
- read configuration files directly

### `yoctui-protocol`

Owns:

- versioned bridge envelopes
- request, response, event, and error wire types
- sequence and correlation identifiers
- framing constraints
- compatibility negotiation data

Protocol changes require backward-compatibility consideration and tests.

### `yoctui-bitbake`

Owns:

- bridge process management
- process backend
- BitBake server adapter
- external Yocto tool adapters
- output normalization
- cancellation and escalation
- workspace queries
- live compatibility boundary

Every adapter returns typed events and typed results.

The production Python bridge uses BitBake's `bb.tinfoil.Tinfoil` client API. It
starts in configuration-only mode for lightweight workspace queries, parses
recipes on demand, and submits `buildTargets` asynchronously. A bridge-side
event pump converts native parse, task, log, completion, and cancellation
records into protocol events. Standard output remains reserved for NDJSON;
BitBake diagnostics go to standard error. The environment-only and mocked
connection paths are test/diagnostic fallbacks and are not live compatibility
evidence.

### Backend event normalization boundary

`yoctui-protocol` owns typed wire payloads, including the complete workspace
snapshot. `yoctui-bitbake` translates each protocol event into a typed
`BackendEvent`; it does not mutate application state. `yoctui-app` is the sole
normalization boundary from `BackendEvent` to reducer `Action` values, and the
model reducer is the sole owner of resulting state changes. Initial discovery
and refresh responses use the same reducer actions as streamed events.

Dependency graphs cross that boundary only as typed recipe/task node identities
and typed build, runtime, or task edges. `yoctui-model` owns deterministic
normalization, identity-stable selection, reverse-edge lookup, and bounded
shortest why-built path derivation. Its state distinguishes not-loaded,
loading, available-empty, available, partial, and failed results. Adapters own
all BitBake, dot, and external-tool parsing; reducers and widgets must never
parse those formats. The legacy direct build/runtime lists remain a temporary
compatibility input while acquisition and workspace tasks migrate to the graph
state.

Capable bridges acquire dependency graphs through BitBake's structured
`generateDepTreeEvent` server command. The Python boundary validates and
bounds the `pn`, `depends`, `rdepends-pn`, `providermap`, and `tdepends`
records, converts them to protocol node/edge data, preserves only absolute
provider/log paths, and reports every dropped field or bound as a limitation.
Protocol version 1 retains the legacy direct-dependencies event; a new graph
command/event is additive, and the Rust bridge client falls back to the legacy
query only when an older peer rejects the new command.

The process backend invokes `bitbake -g <recipe>` directly without a shell,
with a fixed timeout and discarded process output. It removes the known stale
`task-depends.dot` before invocation, accepts only a bounded regular file whose
canonical parent is the active build directory, and parses dot syntax inside
`yoctui-bitbake`. That fallback derives typed task edges and cross-recipe build
edges but explicitly reports runtime edges and provider/log paths unavailable.
Live smoke validation exercises `generateDepTreeEvent` separately from mocked
and fake-process coverage and records the BitBake version and returned graph
families.

Signature dump and comparison state is also model-owned. Exact typed identities
carry recipe, task, optional hash, and an optional absolute authoritative
signature path. The model normalizes bounded records and variable/dependency
data, preserves identity selection, correlates request results, and derives
deterministic typed base-hash, changed-value, dependency, and unavailable-field
differences. Not-loaded, loading, available-empty, available, partial, and
failed outcomes remain distinct for both dump and comparison workflows.
The signature adapter performs a bounded, deterministic scan of the configured
build's `tmp/stamps` tree for exact recipe/task `sigdata` or `siginfo`
artifacts. It rejects symlinks, relative paths, hash/path mismatches, and
canonical paths outside the build directory. It executes `bitbake-dumpsig`
and `bitbake-diffsigs -c never` directly without a shell, drains bounded
standard output and error streams, enforces a timeout, and supports explicit
process-group cancellation. Dump output is normalized into typed values,
task hashes, and task dependencies; comparison output is combined with a
deterministic comparison of the two typed dumps so multiline changes remain
representable. Unsupported recursive `diffsigs` detail is reported as a
limitation. Only typed responses or correlated errors cross the backend/app
boundary; raw tool output and paths inferred from log text never reach reducer
or widget code.

The CLI owns the short-lived Tokio task and cancellation handle for one active
signature dump or comparison. It clones the configured adapter into that task,
continues terminal drawing and input polling, and converts the terminal result
back into a correlated typed `BackendEvent`. The app mapping then emits the
same reducer actions used by backend events; the CLI never parses signatures
or mutates signature state. Leaving an idle Signatures child workspace is a
pure navigation action, while `Esc` during loading signals the adapter and
keeps the correlated loading state until its cancelled result arrives.

Package data state follows the same typed ownership boundary. Exact package
identities key bounded, deterministically normalized inventory summaries and
detail records. A typed field distinguishes unavailable metadata from an
available empty value. Inventory and detail state separately distinguish
not-loaded, loading, available-empty, available, partial, and failed results,
and generation-tagged requests prevent stale responses from replacing newer
data. `yoctui-model` owns package filtering, stable selection, detail caching,
and navigation over typed forward or reverse runtime dependency identities.
The package-data adapter owns all `oe-pkgdata-util` discovery, execution, and
output parsing. It validates a real generated `tmp/pkgdata` directory and
either an explicit tool path or a bounded deterministic `scripts/oe-pkgdata-util`
search beneath the build parent. Symlinks, relative paths, and invalid file
types are rejected. Inventory, package-info, file-list, and runtime-dependency
queries use exact argument vectors without a shell, bounded output and line
counts, fixed-size argument batches, a timeout, and cancellable child process
groups. Forward dependencies come from authoritative `RDEPENDS`; reverse
dependencies are derived from that same bounded inventory. Provider recipe
paths and image membership stay unavailable until an authoritative source
exposes them. Only typed summaries, details, limitations, or correlated errors
cross through `BackendEvent` and the app action mapping.

The CLI owns at most one short-lived package inventory or detail Tokio task and
its cancellation handle. It polls completion alongside terminal input,
rendering, build events, and telemetry, then converts the terminal result into
the same correlated `BackendEvent` used by the app boundary. Entering Packages
starts an inventory only from not-loaded state; refresh and lazy detail effects
use the same coordinator. Navigation never awaits `oe-pkgdata-util`, and stale
generations remain reducer-inert. The model owns bounded package navigation
history plus dependency-kind/identity selection; the UI renders typed state
and emits actions without reading adapter output.

Selected configuration variables use a version-compatible typed detail
payload. It carries global or recipe scope, expanded and unexpanded datastore
values, normalized varhistory operations (`op`, file, line, and detail), and
the active `OVERRIDES` context. The Python bridge is the only component that
interprets Tinfoil varhistory dictionaries. Rust converts paths and stores
detail by `(name, recipe)` identity, so a recipe-scoped or stale response
cannot overwrite the global workspace summary. Older bridge responses default
new fields to unavailable/empty without fabricating history.

Confirmed configuration edits cross the model/CLI boundary as a typed request
containing the global identity, value, exact escaped assignment, and
`conf/local.conf` destination. The CLI revalidates all four fields against the
active build directory, replaces only exact base-variable assignment lines,
and writes a permission-preserving same-directory temporary file before an
atomic rename. It then requests the same typed identity from the backend.
Reducer actions alone own write/refresh lifecycle notifications and cached
detail updates; a failed refresh does not discard the prior detail.

Recipe discovery is split into a bounded summary query and a selected-recipe
detail query. Summary records carry the resolved version, provider path/layer,
and append count from BitBake's provider/cache tables. A typed
`GetRecipeMetadata` request parses only the selected provider and returns
optional tasks, metadata sources/appends, patch URIs, package outputs,
workspace/build state, and history. `None` means the active backend cannot
authoritatively provide that field; an available empty list means BitBake
reported no values. Inventory refresh preserves stable recipe-name selection
and evicts detail state for recipes no longer present.

For local `file://` patch entries, the Tinfoil bridge asks BitBake's fetcher
for the resolved local path in the parsed recipe datastore. Unresolved or
remote entries remain URIs. The model exposes only absolute resolved paths to
the patch-review effect and explains the rest instead of guessing from layer
directory layout. Provider files come from the provider table, while task-log
choices come only from retained typed task records. All three routes emit the
existing editor effect; CLI terminal lifecycle code validates path existence,
restores the terminal, and reports launch or exit failures.

Recipe BitBake operations share a validated model `BuildRequest` containing
typed targets, an optional task, and an explicit force flag. The reducer
derives task choices only from authoritative recipe metadata and emits one
start effect after confirmation. The CLI's build-job coordinator owns
execution, duplicate-job rejection, cancellation, and persistent results.
Process execution translates the request to BitBake arguments; the bridge
serializes the same fields and temporarily applies BitBake's force
configuration only for the corresponding asynchronous build.

The same coordinator classifies `cve_check` and `create_spdx` requests as
distinct CVE/SPDX background-job kinds and records the selected recipe and task
in typed job context. Backend parse, task, log, completion, cancellation, and
disconnect events still drive the single lifecycle used by ordinary builds.
Successful QA completion retains an empty artifact list unless a typed backend
event supplies paths; widgets must describe that as no path reported and must
not extract filesystem locations from log text.

Devtool workspace inspection is a typed external-tool adapter in
`yoctui-bitbake`. It invokes `devtool status` in the active build directory,
normalizes membership and source paths, and then invokes Git porcelain-v2
status only for an existing workspace source. Raw Devtool and Git records do
not cross into the reducer or widgets. Missing executables, missing source
directories, non-repositories, non-zero exits, and malformed records remain
separate model states. The model keys requests and results by recipe name plus
absolute provider path and owns the shared action-availability rules used by
both reducer routing and UI explanations.

Devtool execution requests use one model-owned `DevtoolOperation` enum for
modify, update-recipe, finish, deploy-target, undeploy-target, and reset.
Process-independent validation rejects ambiguous recipe/target tokens and
relative finish destinations. `yoctui-bitbake` alone translates a validated
operation to an executable plus `Vec<OsString>` argument vector; it never
constructs a shell command, and path arguments retain their native OS
representation. Process streaming and job lifecycle are separate layers built
on this command specification.

`DevtoolJobRunner` is the asynchronous process boundary for that specification.
It starts exactly one shell-free child in the active build directory, assigns
a child process group on Unix, and emits typed started, stream-tagged output,
completed, failed, cancelled, and lost events. stdout and stderr use a bounded
channel; individual lines are capped and explicitly marked truncated, with
invalid UTF-8 preserved lossily. Cancellation sends `SIGTERM` to the child
group, waits for a configured interval, then records forced `SIGKILL`
escalation when needed. The adapter neither retains job history nor mutates
application state.

`DevtoolJobCoordinator` maps those runner events into the existing
`BackgroundJobKind::Devtool` reducer transitions. Its stable IDs occupy a
disjoint high namespace from BitBake coordinator IDs, so both job types may
run without state collisions. Retained output carries typed backend, stdout,
or stderr origin and an explicit truncation bit. The CLI owns one runner,
non-blockingly polls it beside backend and keyboard events, and routes `c` to
the active Devtool coordinator before independent BitBake cancellation.
Start, output, success, nonzero exit, cancellation, cancellation rejection,
and loss all use existing background-job actions; no runner mutates model or
widget state directly.

The modify workflow retains the absolute `RecipeIdentity` from its
authoritative eligibility check separately from the process argument. On a
successful typed runner completion, the CLI re-runs `DevtoolInspector` for that
original identity and feeds the refreshed status through the reducer before it
scans or opens any workspace path. The model owns the modify confirmation and
workspace-editor recipe-build transition; `Ctrl+B` therefore produces the
same typed `BuildRequest` confirmation and background BitBake coordinator path
as a Recipes workspace build. Refresh and editor failures update recoverable
model notifications without rewriting the retained Devtool job.

Update-recipe carries the same absolute `RecipeIdentity` from reducer
eligibility through its confirmation and CLI pending-completion state, while
the process adapter receives only the validated recipe token. A successful
`UpdateRecipe` terminal event triggers a new inspection for that stored
identity; non-success terminal events never replace the previously
authoritative status. The refresh result enters through
`Action::DevtoolStatusLoaded`, and refresh errors remain notifications beside
the durable job result.

Finish uses a model-owned `DevtoolFinishPicker` and `DevtoolFinishPlan`.
Picker entries are cloned from typed configured `Layer` records and retain
native `PathBuf` values; the reducer filters relative paths and revalidates the
selected name/path pair immediately before emitting the effect. Only the
bitbake adapter converts the plan's request into the shell-free Devtool
argument vector, preserving non-UTF-8 path bytes. The CLI retains the original
recipe identity separately while the job runs and refreshes it only after a
successful `Finish` terminal event.

Deploy-target uses model-owned `DevtoolDeployDraft` and
`DevtoolDeployPlan` values so the absolute recipe identity cannot be replaced
by navigation or reconstructed from display text. The reducer invokes the
shared `DevtoolOperation` validation before preview and again before emitting
the effect. The adapter receives only the validated recipe/target argument
vector. The CLI retains the original identity until a successful typed
`DeployTarget` event and then refreshes it through `DevtoolInspector`; failure
events do not overwrite prior authoritative status.

Reset carries a model-owned `DevtoolResetPlan` containing the exact absolute
recipe identity and authoritative workspace source slated for removal. The
reducer compares that source against current typed status immediately before
emitting the effect and validates the derived reset operation. The CLI retains
the identity but passes only the validated recipe token to the process
adapter. It refreshes on successful `Reset`; the resulting `NotMember` state is
expected, while process and refresh failures preserve prior status and the
persistent job record.

Unknown future protocol events normalize to an ignored event and do not imply a
backend disconnect. Missing task progress remains unknown rather than becoming
zero. Terminal build events emit one primary build-state action and one
persistent-job lifecycle action. Boundary verification rejects backend,
protocol, and raw JSON dependencies in `yoctui-ui`.

Live task monitoring also subscribes to BitBake runqueue-start events. The
bridge normalizes their copied runqueue statistics into typed queued-task
events, allowing the model to retain BitBake's authoritative completed/total
counts and derive an aggregate waiting count. Recipe task-start events enrich
the same task identity with PID, worker, and source-log details when BitBake
provides them. Widgets render that typed state and never infer details from log
text.

### `yoctui-app`

Owns:

- keyboard and mouse input mapping
- effect orchestration
- background-job execution
- dialog input routing and confirmation effect orchestration
- configuration/session coordination
- editor and inherited-shell launch coordination

It may request reducer actions but must not bypass the reducer to mutate model state.

### `yoctui-ui`

Owns:

- Ratatui rendering
- responsive layout
- theme application
- semantic focus and selection styles
- workspace, inspector, footer, dialog, and notification rendering

Widgets must be deterministic from model state.

### CLI binary

Owns:

- argument parsing
- configuration precedence
- logging startup
- terminal guard lifecycle
- runtime startup and shutdown
- headless command dispatch
- shallow filesystem and Git inspection requested by typed layer-tree effects
- bounded text/binary preview loading
- validated atomic writes for confirmed local configuration effects

The model owns the cached layer tree by stable paths, expansion state,
selection, Git/file metadata, preview classification, and Inspector mode.
Expanding or refreshing emits a directory-specific effect; the CLI reads only
that directory and returns typed entries. File previews are capped at 64 KiB
and include path, text/binary classification, and truncation state. The reducer
rejects a preview whose path is no longer selected. Neither the CLI nor widgets
recursively discover unopened subtrees.

## Dependency direction

The intended direction is acyclic:

```text
model
  ↑
protocol
  ↑
bitbake
  ↑
app
  ↑
ui
  ↑
CLI
```

Support crates may be introduced only when they preserve this separation.

## State flow

```text
terminal/backend input
        ↓
typed Action or typed BackendEvent
        ↓
pure reducer
        ↓
new App state + requested Effects
        ↓
effect executor / background job manager
        ↓
typed result events
        ↓
reducer
        ↓
UI render
```

No backend callback may mutate UI structures directly.

## Background-job model

All long-running operations use one shared job abstraction.

Minimum fields:

- stable job ID
- job kind
- display title
- lifecycle state
- start/end timestamps
- optional target, recipe, task, image, or workspace context
- cancellation capability
- progress representation
- bounded logs
- typed result
- typed error
- artifact references

Lifecycle:

```text
Queued → Starting → Running → Cancelling → Succeeded
                                      └→ Failed
                                      └→ Cancelled
                                      └→ Lost
```

Navigation must not stop a job. Jobs continue while the user changes workspaces.

Indeterminate activity must never imply false numeric progress.

## Log retention and selection

`yoctui-model::LogState` owns byte/entry bounds, protected-record preference,
ordinary-entry coalescing, pause horizons, filters, search, and the selected
filtered index. Warnings, errors, and typed cancellation/disconnect/final
records are protected from eviction while ordinary records remain. If the
configured bound contains only protected records, the oldest record is evicted
and its severity counter remains observable.

Each retained log carries its typed build target, recipe, task, source path,
timestamp, and protection state. `yoctui-ui` renders only the selected typed
entry in the Inspector. Source opening and clipboard copying are typed effects
executed by the CLI; clipboard execution probes `wl-copy`, `xclip`, then `xsel`
without invoking a shell and reports unsupported environments visibly.

Warnings and errors additionally carry a stable retained ID and typed
`DiagnosticInfo`: normalized category, bounded summary, event metadata, and
suggested actions. The Errors workspace derives from these diagnostic records,
not severity-colored text parsing. Exact log navigation stores the diagnostic
ID as a temporary jump target, preserving the user's query and filters while
making that one retained entry selectable. Completion and backend-loss
reducers create typed protected diagnostics and actionable outcome state.

## Dialog architecture

Dialogs are typed model values, not ad-hoc widget-local state.

`yoctui-model::App` owns a FIFO dialog queue. The front value is the only
active dialog and carries every field required by that workflow. Reducer
actions explicitly open, replace, confirm, cancel, or dismiss that value.
Asynchronous completion can enqueue behind an active user dialog, so backend
events never interrupt or discard in-progress input.

`yoctui-app` maps input for the active variant and executes returned effects.
`yoctui-ui` renders only the active variant. Neither layer establishes its own
dialog precedence or mutates dialog state directly.

Each dialog defines:

- purpose
- fields
- validation
- confirmation strength
- accepted action
- cancelled action
- focus order

Modal dialogs trap focus. Destructive actions show the exact command or configuration change before confirmation.

## Command catalog architecture

The command palette is a typed model-owned catalog. Each entry has a stable
identifier, label, description, shortcut, deterministic order, and optional
disabled reason derived from current model context. The reducer owns the
query, filtered selection, activation, and focus transitions. Disabled or
empty activation is inert; enabled activation dispatches the same typed action
used by the corresponding shortcut.

The application layer maps palette keystrokes, the CLI routes them before
workspace input, and the UI renders filtered model entries. Neither input nor
rendering code maintains a separate command list or availability rule.

## Tool integration contract

Each Yocto tool integration should contain:

1. capability detection
2. typed input model
3. validation
4. preview
5. execution adapter
6. typed progress and logs
7. typed result
8. cancellation where possible
9. workspace/inspector presentation
10. fake integration tests
11. live validation when required

An unstructured shell textbox may be offered as an escape hatch, but it is not the primary UX for required tools.

## Error model

Errors should preserve:

- source component
- job/build/task context
- timestamp
- severity
- human-readable summary
- bounded detailed text
- source path and line when available
- suggested navigation target
- underlying exit code or protocol error

UI rendering must not infer error types from raw strings.

## Configuration and persistence

Precedence:

```text
startup/runtime fields:
CLI > YOCTUI_* environment > config.toml > session.toml > built-in defaults

interactive visual/log preferences:
CLI hard overrides > session.toml > config.toml defaults > built-in defaults
```

The model owns typed Settings selection, immediate preview state, and a dirty
bit. A settings change returns a persistence effect. The CLI merges only the
supported preference fields into a cloned session value and atomically
replaces `session.toml`; it never rewrites `config.toml`. Successful writes
clear the dirty bit. Failed writes leave the previewed value and dirty state
intact and dispatch a visible failure notice.

Persist only user preferences and recent valid workspace references. Do not
persist transient secrets or unbounded logs.

## Terminal ownership

Terminal initialization and restoration use RAII. Restoration includes:

- raw mode
- alternate screen
- cursor
- mouse capture
- bracketed paste
- panic and supported termination paths

Inherited shell and external editor transitions must temporarily restore terminal state and then reconstruct it safely.

## Testing boundaries

- model: unit and property tests
- protocol: framing, compatibility, malformed and oversized input
- bitbake: fake process, fake bridge, mocked BitBake modules, cancellation
- app: effect and input mapping tests
- UI: `TestBackend` semantic snapshots and responsive dimensions
- CLI: integration tests and pseudo-terminal tests
- live: supported Yocto smoke matrix

## Compatibility claims

Mocked tests prove adapter logic, not live compatibility.

A Yocto/BitBake release may be listed as supported only after:

- workspace inspection
- variable, recipe summary/detail, and layer queries
- build start
- task and parse event normalization
- normal completion
- cancellation
- bridge shutdown

are exercised in a real initialized environment.

The repeatable opt-in entry point for this boundary is
`scripts/verify-live-bitbake.sh`. It validates preconditions before starting
BitBake and records the tested matrix in `docs/compatibility.md` only after the
full cycle succeeds.
