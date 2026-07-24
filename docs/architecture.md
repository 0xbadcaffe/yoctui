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

### `yoctui-app`

Owns:

- keyboard and mouse input mapping
- effect orchestration
- background-job execution
- dialogs and confirmations
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

## Dialog architecture

Dialogs are typed model values, not ad-hoc widget-local state.

Each dialog defines:

- purpose
- fields
- validation
- confirmation strength
- accepted action
- cancelled action
- focus order

Modal dialogs trap focus. Destructive actions show the exact command or configuration change before confirmation.

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
CLI
> YOCTUI_* environment
> config.toml
> session.toml
> built-in defaults
```

Persist only user preferences and recent valid workspace references. Do not persist transient secrets or unbounded logs.

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
