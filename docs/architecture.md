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
- variable, recipe, and layer queries
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
