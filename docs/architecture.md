# Yoctui Architecture

## Purpose

Yoctui is a Rust/Ratatui terminal workbench for Yocto and BitBake. BitBake remains the authority for metadata and build state. Yoctui requests operations, normalizes events, stores bounded state, and renders typed views.

Editable workflows use a model-owned bounded popup-document state. The model
serializes typed drafts to TOML and validates parsed values before emitting the
existing typed effects; the UI renders only the document/mode and the CLI maps
vi-like input. This keeps parsing and validation out of Ratatui widgets and
preserves each workflow's confirmation boundary.

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

Deployed image artifact state is model-owned and keyed by the exact effective
machine, image target, and absolute deployed path. A typed field distinguishes
unavailable metadata from an available empty collection. Inventory lifecycle
distinguishes not-loaded, loading, available-empty, available, partial, and
failed results, while non-zero request generations make stale responses
reducer-inert. Deterministic normalization rejects mismatched machine
identities, relative or non-normalized paths, and paths outside an
authoritatively supplied deploy directory. It also bounds artifact,
associated-file, checksum, limitation, and search state. Artifact kind, byte
size, modification timestamp, checksum records, manifest paths, license paths,
SPDX/SBOM paths, Wic-related paths, and the deploy directory cross the app
boundary only as typed data. The model owns search and identity-stable
selection; it never scans the filesystem, and widgets must not classify file
names or parse metadata.

The image artifact adapter receives the exact `DEPLOY_DIR_IMAGE` value acquired
by Tinfoil (or the environment-only diagnostic fallback) and the typed machine
request. It requires an absolute, non-symlink directory whose canonical leaf
matches that machine. A blocking filesystem worker performs a deterministic
depth-one scan under entry, checksum-byte, checksum-line, and elapsed-time
bounds while consulting a cancellation token between records. It never follows
deploy symlinks or accepts a canonical file outside the configured directory.
Only the adapter classifies root filesystem, kernel, bootloader, Wic,
manifest, license, SPDX/SBOM, checksum, and other files and parses bounded
checksum records. Malformed, oversized, nested, symlinked, missing, or
unassociated data becomes a typed error or explicit limitation. The response
converts directly to `BackendEvent::ImageArtifacts`; raw directory entries and
checksum text never cross into `yoctui-app`, reducers, or widgets.

The CLI constructs the image adapter only from the typed
`DEPLOY_DIR_IMAGE` workspace variable and owns at most one short-lived scan
task plus its cancellation token. Entering Images from not-loaded state and
explicit refresh both consume the same reducer effect. Polling completion
converts the typed response/error through the app normalization boundary;
missing configuration becomes a correlated failed result, and stale
generations remain reducer-inert. Rendering, keyboard input, telemetry, and
BitBake jobs continue while scanning. Exact artifact and associated paths use
the existing editor lifecycle, while selected-image builds reuse the normal
confirmed `BuildRequest` and persistent BitBake job coordinator.

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
Before that typed request is created, the model owns the bounded popup TOML
document and its Normal/Insert mode for both configuration values and BBMASK;
the UI only renders it and the CLI maps editor input and paste to typed reducer
actions.

All TOML popup workflows share model-owned cursor, selection, and edit-mode
state. The UI never owns text editing state: it renders the current document,
cursor, selection, and common shortcut row; the CLI maps keys, paste, and copy
to typed editor actions; workflow reducers only serialize and validate their
typed drafts.

The popup editor remains a small model-owned adapter rather than storing a
`tui-textarea` widget in UI state. `tui-textarea` informed the supported
editing contract (cursor movement, selection, copy/paste, and line bounds),
but its widget-owned mutable state would violate Yoctui's reducer boundary.

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

## Managed SDK boundary

`yoctui-model::sdk` owns SDK kind, exact machine/distro/image/artifact
identities, generation-correlated inventory state, selection/search, typed
populate/test requests, publication/native-tool drafts and exact previews, and
stable SDK operation context. It may reuse `BuildRequest` and the shared
background-job lifecycle, but it never reads deploy directories, locates host
tools, sources an environment script, parses process output, or launches a
child.

`yoctui-bitbake` owns canonical bounded scanning beneath the authoritative
BitBake-reported SDK deploy root and classifies regular non-symlink installers,
checksums, manifests, and other records. A separate SDK tool adapter resolves
`oe-publish-sdk`, `oe-find-native-sysroot`, and `oe-run-native`, validates exact
artifact/destination/extracted-root identities, reconstructs shell-free native
argument vectors independently from model previews, builds a child-only
environment from validated typed data, and emits bounded stream-tagged runner
events. It never mutates the Yoctui process environment or invents metadata
from display names.

`yoctui-app` maps SDK keys and adapter events mechanically. `yoctui-ui` renders
only typed SDK state. The CLI routes populate/test tasks through existing
managed BitBake execution, owns at most one replaceable artifact scan and one
SDK tool runner, polls both without blocking terminal input, and refreshes the
exact correlated inventory after successful generation/publication. The
Testing workspace may consume SDK test results later, but it does not duplicate
SDK launch state.

## Managed Testing boundary

`yoctui-model::testing` owns test-family and selector types, exact active
machine/distro/image/configuration identity, validated launch drafts and
previews, stable `TestSession` context, result identities, bounded normalized
suite/case records, selection/search, comparison categories, export requests,
and correlated lifecycle state. It may reuse `BuildRequest` and the shared
background-job collection, but it never discovers executables, reads result
JSON, parses process output, walks the filesystem, launches a child, or writes
JUnit.

Image runtime, SDK, extensible SDK, and configured ptest launches are exact
BitBake tasks owned by the existing build coordinator. The Testing model does
not duplicate SDK launch state: an SDK task may originate from SDK or Testing,
while exact structured results and comparisons are consumed in Testing.
Ptest is exposed as a configured image-runtime suite only when typed
configuration proves its prerequisites; no component guesses a target or
silently mutates build configuration.

`yoctui-bitbake` owns canonical discovery and independent revalidation of
`oe-selftest`, `bitbake-selftest`, and `resulttool`; exact shell-free
construction; child-only environment entries; one process-group-owned test
runner; and bounded stream-tagged lifecycle events. Its result adapter accepts
only canonical explicit or managed-session result roots, refuses symlinks and
escapes, parses bounded `testresults.json` data into typed records, invokes
supported resulttool operations with exact indexed vectors, and validates a
non-overwriting JUnit destination immediately before spawn. Raw JSON,
resulttool report text, unittest output, and filenames never cross into
widgets as authority.

`yoctui-app` maps Testing keys and adapter events mechanically.
`yoctui-ui` renders only typed Testing state and emits typed actions. The CLI
owns capability inspection, at most one selftest runner, replaceable
generation-correlated result imports, and one comparison/export operation. It
routes BitBake families through the existing coordinator and polls every
Testing-owned operation without blocking terminal input. Completion, refresh,
cancellation, timeout, nonzero failure, rejection, and worker loss are mapped
once into reducer actions.

Mocked process/filesystem coverage proves only these boundaries. Live support
requires opt-in execution in an initialized compatible Yocto environment and
must record the exact release, tool capabilities, selected test, result
identity, and outcome without claiming external target or ptest coverage that
was not exercised.

## Managed Security boundary

`yoctui-model::security` owns exact recipe/image scope, capability state,
release-dependent task choices supplied as typed data, stable operation and
report-generation identities, CVE findings and package mappings, SPDX
document/component summaries, bounded typed mapper stream retention,
search/filter/drill selection, previews, background-job association, and
correlated lifecycle state. Its explicit report import dialog owns the shared
bounded `PopupEditor` document and validation state; the app maps keys to
typed editor/security actions and the UI only renders that state. It may reuse
`BuildRequest` and the shared background-job collection, but it never inspects
the host, guesses tasks or report roots, walks directories, parses reports or
process output, launches a child, or opens a path/URL.

Security BitBake requests use the existing managed build coordinator.
Capability input comes from authoritative recipe tasks and typed BitBake
configuration: the model never assumes that `cve_check`, legacy
`create_spdx`, current `create_recipe_sbom`, image SBOM generation, or package
mapping is available based on a release name. Successful operations cause an
exact generation-correlated report refresh; success without a typed artifact
or newly observed report remains explicit.

`yoctui-bitbake` owns canonical capability discovery, report-root and
executable validation, exact shell-free package-mapping vectors, bounded
filesystem acquisition, content fingerprints, CVE/SPDX parsing, and immediate
identity revalidation before spawn or open. It accepts only authoritative
roots, managed-job artifact paths, or explicit imports; refuses symlinks and
escapes; and bounds directory entries, files, bytes, records, fields, and
parse time. The package mapper runs as one shell-free native-argv child in its
own process group, revalidates its exact executable and input identities before
spawn, bounds both streams, and exposes correlated start, output, success,
nonzero, cancellation, timeout, and loss events. Unknown CVE statuses and
unsupported SPDX schemas become typed
unknown/partial data with limitations rather than guessed meaning. Raw JSON,
text, archives, filenames, and tool/log output never cross into widgets as
authority.

`yoctui-app` maps Security keys, adapter responses, and runner events
mechanically. `yoctui-ui` renders only typed capability, findings, document
summaries, lifecycle, and dialog state and emits typed actions. The CLI routes
CVE/SBOM builds through the managed BitBake coordinator, owns at most one
independent package-mapping runner and one replaceable generation-correlated
report worker, polls them without blocking terminal input, and routes opens
only after adapter revalidation. Cancellation is correlated to the exact
Security operation and cannot target unrelated work.

Fake process/filesystem coverage proves boundary behavior only. Live Security
support requires opt-in execution against an initialized compatible Yocto
environment and must record the exact release, capability/task names, scope,
report identities, and outcome. A mocked legacy/current task matrix or parsed
fixture alone is not a live compatibility claim.

## Managed QA boundary

`yoctui-model::qa` owns the typed Recipe & Kernel and Layer QA views, exact
recipe/provider and configured-layer scopes, capability-supplied check
catalog, stable operation/session/report/finding identities, deterministic
previews, search/filter/drill selection, bounded retained session output,
managed-job association, dialog state, cancellation, and correlated lifecycle
outcomes. Its explicit report import dialog owns the shared bounded
`PopupEditor` document and validation state; the app maps keys to typed
editor/QA actions and the UI only renders that state. It may reuse
`BuildRequest`, `RecipeIdentity`, `Layer`, and the shared
background-job collection, but it never inspects the host, guesses a task or
tool, walks report directories, parses logs/reports/process text, launches a
child, or opens a path.

Each catalog entry is typed as kernel configuration, URI, patch, license,
general recipe/package, or configured-layer QA and carries an execution kind
plus an exact availability reason. Recipe/kernel operations use only tasks
reported for the exact provider and reuse the existing managed BitBake
coordinator. Task strings such as `kernel_configcheck`, `checkuri`, or
`package_qa` are capability values, not release-derived defaults. The model
cannot enable inherited classes or translate an unavailable check into an
arbitrary shell command.

`yoctui-bitbake` owns fail-closed capability construction from explicit
initialized metadata, canonical report/tool/layer validation, bounded report
acquisition and content fingerprints, normalized QA finding parsing, and the
exact `yocto-check-layer` adapter. The layer runner captures and immediately
revalidates the canonical executable and configured-layer identities,
reconstructs only the confirmed indexed vector, uses native argv in its own
process group, and emits typed bounded start/output/completion/nonzero/
cancellation/timeout/loss events. Report adapters accept only exact roots or
imports, refuse symlinks and escapes, bound traversal/files/bytes/records/
fields/time, and preserve partial valid data with limitations. Unknown formats
or records remain unknown/partial instead of being inferred from filenames,
colors, or unstructured output.

The documented QA report formats are a JSON `findings` envelope, a JSON array
or JSON-lines stream of finding objects; a `qa-report` XML envelope containing
self-closing `finding` records; tab-separated text whose first fields are
status and message followed by bounded `key=value` fields; and exact BitBake
log records beginning `ERROR: QA Issue:` or `WARNING: QA Issue:`. Candidate
files carry an explicit matching format, while bounded directory imports use
only the allowlisted `.json`, `.jsonl`, `.xml`, `.qa`, `.txt`, and `.log`
suffixes. Unsupported records stay limitations and never become findings.

Normalized findings carry stable check and finding identity, exact scope,
typed status/severity, bounded message, optional authoritative source path and
line, rule/code, suggestion, and report identity. Raw BitBake logs, native
stdout/stderr, report JSON/XML/text, and filenames never cross into widgets as
authority. Successful managed operations without an exact report remain
successful with no report and do not fabricate findings.

`yoctui-app` maps QA keys, adapter responses, and layer-runner events
mechanically. `yoctui-ui` renders only typed QA capability, catalog, report,
finding, session, limitation, and dialog state and emits typed actions. The
CLI snapshots the initialized build directory and child-only executable
search path, routes recipe/kernel builds through the managed BitBake
coordinator, owns at most one independent layer-QA runner and one replaceable
generation-correlated report worker, and polls them without blocking terminal
input or unrelated operations. It revalidates exact reports/providers/finding
sources before editor launch and correlates cancellation to the exact QA
session only.

Existing Recipes task execution and local patch review remain contextual
routes over the same authoritative recipe identity; the QA destination does
not fork their editor or build lifecycle. UI behavior changes are specified in
`docs/ui-spec.md`, while the backend remains the authority for task, provider,
layer, and report identity.

Fake process/filesystem coverage proves only these boundaries. Live QA support
requires opt-in execution in an initialized compatible Yocto environment and
must record the exact release, capability/task/tool names, recipe or layer
scope, report identities, and outcome. Fixture output from
`yocto-check-layer` or a mocked task catalog is not live compatibility
evidence.

## Managed Maintenance boundary

`yoctui-model::maintenance` owns the fixed Sstate, Services, Release, and
Integrations views; capability, metadata, and optional-integration snapshots; canonical input and
evidence identities; typed operation drafts and confirmations; stable
operation sessions; bounded output; service diagnostics; and all selection,
focus, and terminal outcome state. It contains no host inspection, filesystem
access, raw process parsing, or command construction. Existing Signatures,
Security, QA, and recipe patch-review state remains authoritative and is
reached through typed navigation actions.

The Sstate readiness and cleanup entry points own shared bounded `PopupEditor`
documents in model state. Parsing accepts named TOML fields, native booleans
and integers, and exact readiness mode values, then converts them into the
pre-existing typed requests. Cleanup cache and stamps identities always come
from current capability metadata rather than editable document content.
Invalid documents stay in the popup with visible validation; valid documents
still cross the adapter's capability-derived preview boundary and never create
an execution path in UI or input-routing code. Exact cleanup candidate
discovery, phrase entry, and destructive confirmation remain separate stages.

`yoctui-bitbake` owns four adapter families. The sstate adapter detects the
installed readiness and cleanup interface, constructs validated native vectors,
previews exact cleanup candidates, revalidates the candidate set and canonical
cache/stamps identities, and executes only a confirmed typed request. The
service adapter acquires configured PR/hash variables, performs bounded
observational endpoint/process diagnostics, and exposes only documented
installed PR-tool operations; it never owns service lifecycle. The release
adapter validates locked-signature inputs, build-history revisions, archive
roots, and evidence, then runs shell-free commands with bounded output. The
optional adapter reports pull-request, error-report, repo-manifest, and Toaster
capability without network or lifecycle side effects.

Locked-cache popup validation reconstructs the native LSB identity from current
capability metadata and accepts only typed path fields from the shared editor.
The established adapter preview, changed-evidence detection, and separate
destructive confirmation remain downstream requirements.

Build-history popup validation likewise reconstructs the canonical repository
from current capability metadata, parses only revision/exclusion strings and
typed report booleans, and emits the existing bounded comparison preview.

Git-archive popup validation maps bounded string fields and native booleans
into the established typed archive request. A remote remains intent inside the
local request: local evidence must complete before the separate network-push
confirmation can be offered.

Service diagnostics cross the boundary as typed endpoint-role, local/remote,
reachability, PID, and executable-name records rather than raw process or
socket text. The adapter bounds an explicit process-root scan and records
`bitbake-prserv`, `bitbake-hashserv`, and `bitbake-worker` names only as
observational evidence. Numeric, localhost, and UNIX endpoints may receive a
bounded connection probe; named remote endpoints remain configured with an
explicit unprobed limitation rather than invoking unbounded name resolution.
Typed endpoint observations allow deterministic worker and fixture input but
cannot turn process-name evidence into service health. PR export/import retains
the exact build directory and configured endpoint, revalidates the canonical
helper plus source/destination identities, and reuses the same Maintenance
process-group runner as sstate operations. Its shared popup carries only the
selected operation and editable `file` field; current build-directory and
endpoint identities are reconstructed from capability metadata during
validation, so edited informational comments cannot change execution context.

Release commands use the same runner through a guarded external-command
specification. `gen-lockedsig-cache` retains its ordered positional interface
and an exact pre-operation output inventory, so only created or identity-changed
regular evidence is installed after success. `buildhistory-diff` retains its
documented repository, reporting, signature, exclusion, colour, and revision
arguments; the repository and HEAD identity are revalidated before spawn. A
detected optional `build-compare` remains unavailable until its distinct native
interface is explicitly supported and is never translated into a
`buildhistory-diff` request.

`oe-git-archive` local creation is constructed without `--push`, even when the
typed user request includes a remote. A push vector can be constructed only
from a separately captured and revalidated local repository/HEAD result, and it
retains the remote as an explicit network side effect. Data, repository, note,
tool, and local-result identities are guarded independently; fake repositories
and helpers establish vector and lifecycle behavior only.

The optional-integration adapter is deliberately inspection-only. Its typed
result is normalized into the model-owned integration snapshot before UI
rendering. It resolves
the four named helpers through bounded child search paths and records exact
regular non-symlink executable identities. Pull-request readiness additionally
requires a canonical Git worktree and HEAD identity; error-report readiness
retains one explicit canonical report candidate. Repo-manifest readiness keeps
the `repo` executable, workspace directory, and manifest target as separate
identities and accepts a manifest link only when its canonical regular target
remains beneath that workspace's `.repo` directory. Toaster readiness retains
canonical configuration files plus bounded process-name evidence. An observed
`toaster` or `toaster-eventreplay` process is diagnostic only and cannot turn a
missing executable/configuration into an available capability. The adapter
constructs no command and performs no mail, upload, manifest mutation, network,
or service-lifecycle action.

All Maintenance adapters return typed capability, preview, diagnostic,
progress, result, and error values. Raw output parsing and filesystem/process
classification stop at this boundary. A missing executable, metadata field, or
unsupported tool generation is an explicit unavailable capability. In
particular, legacy `sstate-cache-management.sh` and current
`sstate-cache-management.py` are different interfaces, and `build-compare`
does not alias `buildhistory-diff`.

`yoctui-app` maps Maintenance keys and adapter events mechanically.
`yoctui-ui` renders only typed state and emits typed actions. The CLI owns
replaceable capability/service workers and at most one independent Maintenance
operation runner, polls all of them without blocking terminal/backend input,
and revalidates exact capability, destructive candidate, and evidence identity
immediately before side effects. Managed BitBake work continues to use the
shared build coordinator.

The CLI coordinator builds one immutable inspection context from canonical
initialized-workspace metadata and bounded child-only executable paths. A
replacement blocking worker inspects all four adapter families and returns
separately correlated capability, service, and optional-integration results;
adapter failure remains an explicit unavailable capability or failed typed
diagnostic. Confirmed operations trigger a fresh inspection before command
reconstruction. The coordinator rejects any preview or capability change,
runs cleanup discovery before reconstructing its destructive command, and
retains local Git archive evidence separately from a later push request. One
runner task owns output, deadlines, and the process group; cancellation moves
that exact runner into an asynchronous wait so terminal and BitBake input stay
responsive. Successful post-run evidence validation replaces model evidence,
while validation or command failure preserves the previous evidence set.

Sstate form effects enter the coordinator before confirmation. Readiness
reconstructs and exposes the adapter-owned exact vector without spawning it.
Cleanup uses a separate non-destructive preview runner whose bounded stdout is
parsed only by the sstate adapter; only a successfully parsed exact candidate
set enters the phrase dialog. The preview runner and destructive operation
runner are mutually exclusive, and execution rediscovers and compares the
candidate set again immediately before deletion.

Cleanup, PR import/export, locked-cache output replacement, repository
creation/tagging, and optional remote push are separate typed side-effect
classes. A cleanup request requires both its exact candidate preview and phrase
confirmation; a remote push can occur only after a separately confirmed local
archive result. Cancellation, timeout, nonzero exit, process loss, stale
correlation, and output truncation remain distinct reducer outcomes. Successful
replaceable evidence is installed atomically; a failed attempt does not erase
prior evidence.

Fake adapters establish boundary behavior only. Live cache safety, service
health, PR data compatibility, locked-cache correctness, build comparison,
archive correctness, and network interoperability require explicit initialized
Yocto validation. Destructive and network validation must use disposable,
opt-in resources and cannot be inferred from unit fixtures.

## Managed runqemu model boundary

`yoctui-model::qemu` owns capability states, exact artifact-bound launch
requests, editable drafts, deterministic argument previews, stable session
identity, and validation. It accepts only normalized absolute executable and
artifact paths supplied by typed capability inspection. The pure reducer never
looks for `runqemu`, reads the filesystem, parses process output, or owns a
terminal.

A `QemuSession` associates the validated request with a disjoint
`BackgroundJobKind::Qemu` ID. The shared background-job collection remains the
single owner of queued, starting, running, cancelling, succeeded, failed,
cancelled, and lost lifecycle state; timestamps; bounded stream-tagged output;
and typed result/error details. The session retains QEMU-specific identity and
exit information without duplicating lifecycle storage. Active-session checks
are derived from that shared job state.

`yoctui-bitbake::QemuRunnerEvent` is the typed adapter-to-application event
boundary. `yoctui-app::qemu_actions_for_runner_event` performs only mechanical
event normalization to QEMU reducer actions. `QemuCapabilityInspector` accepts
an explicit executable candidate or the active `PATH`, resolves it to a
canonical executable, and correlates only canonical regular non-symlink
root-filesystem/Wic files whose exact path and machine identity agree. Missing
tool, missing compatible artifact, and failed inspection remain separate
capability states.

`QemuCommandSpec` revalidates the model request and every filesystem path, then
requires the deterministic preview arguments to exactly equal its independent
shell-free `Vec<OsString>` translation. `QemuJobRunner` owns one child in the
active build directory and a Unix child process group. Its bounded channel and
64 KiB line reader emit starting/started, stream-tagged output with explicit
truncation, success/nonzero failure, cancellation, and loss events. Cancellation
sends `SIGTERM`, waits a bounded interval, and records forced `SIGKILL`
escalation. Neither inspector nor runner retains application history or mutates
model/widget state. Widget rendering, effect coordination, and input mapping
remain in the QEMU UI child task; fake runner tests do not establish live
runqemu compatibility.

The CLI owns one optional managed QEMU operation independently of BitBake,
Devtool, signature, package, and image-artifact operations. After every
successful, partial, or empty image-artifact inventory update, it runs
`QemuCapabilityInspector` against that exact normalized inventory; a refresh or
failed inventory clears stale availability. Launch execution reconstructs the
typed preview, creates `QemuCommandSpec`, and starts `QemuJobRunner` in the
active build directory. The terminal loop polls its bounded event stream
without waiting for process completion and normalizes events through
`yoctui-app`; confirmed cancellation runs in a separate Tokio task so terminal
input and other coordinators remain responsive. CLI ownership is released only
after a terminal runner event or a lost cancellation task, while completed
session history remains in the model.

## Managed Wic boundary

`yoctui-model::wic` owns exact machine/image/kickstart/output/device identities,
bounded kickstart partition summaries, deterministic creation and write
previews, the bounded Wic TOML popup document and its Normal/Insert state,
generated-output inventories, and stable Wic job context. It reuses the shared background-job lifecycle but does not discover
executables, parse raw command output, inspect files or block devices, or
authorize privilege escalation.

The Wic adapter owns canonical executable and kickstart discovery, bounded
kickstart reading/parsing, independent shell-free argument construction,
process-group execution, and canonical post-create output scanning. Creation
initially supports Wic cooked mode only. The adapter also owns bounded block
device discovery and immediately-before-spawn revalidation of the image,
whole-device identity, removability, writability, capacity, mount descendants,
and system/root-device exclusion. It executes `wic write` only when every typed
safety invariant still agrees and never invokes `sudo`.

Device discovery executes canonical `lsblk` directly with the fixed JSON,
byte-size, full-path, and explicit-field argument vector. Both streams, elapsed
time, record count, path and metadata lengths, mount count, candidate count,
and reported limitations are bounded. Every recursive record is validated
before filtering. Exactly one top-level whole-disk subtree must contain the
current `/` mount; otherwise discovery fails closed. Only other top-level
`disk` records backed by canonical non-symlink block nodes that can be opened
for writing may cross the adapter boundary. Partitions, loops, device-mapper
and optical records, mounted descendants, read-only/non-removable devices, and
undersized devices are retained only as bounded exclusion explanations.

The write runner accepts a model-confirmed `WicWriteRequest`, then repeats
canonical image inspection and the complete device discovery immediately
before spawning. The rediscovered identity must exactly match path,
major/minor, capacity, model, serial, and transport as well as every eligibility
flag. It then reuses the one-child kill-on-drop Wic runner with the exact native
arguments `write`, image path, and device path; write completion deliberately
does not run the creation-output scanner. Test-only fake-node policy exercises
the safety classes without treating fake paths as live removable-media
validation.

App code maps keys and typed adapter events mechanically. Ratatui widgets render
typed capability, partition, output, device, and job state without parsing
kickstart text, process output, or device command output. The CLI owns at most
one Wic process operation and one short-lived discovery operation, polls them
independently from BitBake, Devtool, QEMU, and metadata work, and releases
runner ownership only after a terminal event. Exact typed-phrase and command
previews are model gates; the adapter's final revalidation is the authoritative
write gate. Fake device/process coverage is not live removable-media evidence.
Write startup owns that final adapter revalidation in a separately polled Tokio
task, so bounded `lsblk` inspection never blocks terminal input. The same
operation owner transitions the validated runner into normal polling, reports a
lost startup task distinctly, and can abort a still-pending revalidation when
the user confirms the incomplete-device cancellation warning. Synthetic block
nodes are enabled only by the adapter's `test-fixtures` feature for downstream
integration tests; production construction always validates canonical writable
block-device nodes.

The model resolves a write source from the exact selected generated output
before the selected deployed artifact and accepts only typed Wic/direct records
whose native file name is exactly uncompressed `.wic` or `.direct`. Deployed
artifact acquisition classifies those exact suffixes as Wic and no longer
classifies compressed `.wic.*` files as write-capable. Size and modification
identity must be authoritatively available before discovery.

A non-zero device generation and full image identity correlate each inventory.
The model retains selection by the complete device identity, not display path,
and owns the picker, bounded exact-phrase entry, separate command preview, and
write-specific incomplete-device cancellation warning as typed dialog variants.
Preview and final confirmation independently reconstruct from current
capability, image, inventory request, and selected device state. Selection,
phrase entry, or a stale dialog alone can never enqueue a write session.

The workspace bridge includes expanded `WKS_FILE`, `WKS_FILES`,
`WKS_SEARCH_PATH`, and `WKS_FILES_DIR` values when BitBake reports them. The CLI
accepts only absolute configured kickstart files and absolute search roots from
those typed workspace values; relative or absent values are never expanded by
guessing. Capability inspection runs in a generation-correlated Tokio task so a
stale image inventory cannot replace current Wic capability. Both inspection
and creation children use kill-on-drop process ownership. Creation reconstructs
the typed preview from the retained request, requires the adapter's independent
exact-argv revalidation, then polls starting, running, bounded stream output,
completion/output scan, failure, cancellation, rejection, and loss events
without blocking the terminal loop.

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

## Optional project-profile boundary

`.yoctui/project.toml` is an optional, repository-owned declaration of team
intent. `yoctui-model` owns its versioned pure domain representation:
`ProjectProfile`, typed recipe/image/layer favorites, typed build presets,
allowlisted workflow steps, portable relative references, validation results,
and explicit resolved/stale/ambiguous identity states. No profile model type
contains a free-form command, shell fragment, environment assignment, secret,
or host-absolute path.

Schema version 1 is represented by `ProjectProfile`, `ProjectFavorites`,
`ProjectBuildPreset`, `ProjectBuildOptions`, `ProjectWorkflow`, and the closed
`ProjectWorkflowStep` enum. `PortableProjectPath` validates during construction
and deserialization; `ProjectIdentityResolution<T>` keeps resolved, stale, and
ambiguous results distinct for later authoritative reconciliation.

The CLI owns bounded file discovery beneath the canonical project root and
read-only TOML decoding. It rejects symlink escape and unsupported schemas and
passes typed data into the model. Loading has no execution effect. The app
resolves logical identities through existing authoritative metadata actions
and adapters; it does not create a second recipe, image, layer, or build-state
catalog. A profile remains team intent while BitBake-derived state remains
authoritative.

Configured startup selects the project root from the validated `OEROOT` when
present, otherwise from the configured build directory's parent until the
onboarding profile supplies a more exact source root. The CLI treats a missing
file as `Absent`, caps input and generated output at 1 MiB, rejects symlinked
profile directories/files, and reports parse/schema failures as typed invalid
state without preventing ordinary Yoctui startup.

Workflow steps reference a closed enum of existing typed Yoctui actions.
Activation produces the same previews, capability checks, correlations,
confirmations, background-job effects, and destructive-operation policies as
manual navigation. A sequence pauses at each required user decision and never
executes merely because the file was loaded. Unknown step kinds and stale or
ambiguous inputs fail closed.

`project_profile_items` derives renderable resolved, stale, ambiguous, and
unavailable rows from profile intent plus the current authoritative workspace
and image inventory. The reducer may navigate a resolved favorite or create
the existing build-confirmation dialog for a resolved preset; it never emits a
build effect from profile activation. Workflow rows remain inert until their
typed steps are reviewed through the normal action-specific boundaries.

Profile generation is a separate explicit typed effect. It serializes a
minimal deterministic current-schema document to a reviewed destination using
atomic replacement rules. Existing files require replacement confirmation.
Personal settings stay in user-local `config.toml`/`session.toml` and are never
imported into or overridden by a project profile.

The write adapter creates `.yoctui` only beneath a canonical project root,
uses a create-new temporary regular file plus sync, refuses an existing target
unless replacement was explicitly confirmed, and never follows a profile or
profile-directory symlink. The reducer retains a generation preview on write
failure and installs the loaded profile only after success.

The `profile` CLI inspection route uses the same bounded loader, reducer, and
selected backend as the interactive client. It loads live workspace, recipe,
and layer inventories before reporting every team-intent item as resolved,
stale, ambiguous, or unavailable. The bridge keeps a duplicated protocol file
descriptor and redirects ordinary process stdout to stderr before BitBake is
initialized, so BitBake startup diagnostics cannot contaminate the bounded
NDJSON protocol stream.

### Build-environment onboarding

Interactive startup may begin without a build directory. `yoctui-model` owns a
typed environment profile and lifecycle for source inspection, clone preview,
initialization, interactive-shell handoff, and connection verification. It
does not parse shell output or infer success from a prompt. Until a correlated
typed workspace response succeeds, it keeps the workspace disconnected and
build-capable actions disabled.

The Build environment workspace is a first-class `Screen`/Navigator entry,
separate from visual Settings. Its verified profile owns the active source,
build directory, captured child environment, and typed image inventory. A
profile replacement invalidates the active backend and image list before the
next initialization/verification cycle.

`yoctui-bitbake` owns source/build/script validation and the bounded setup
adapter. It executes only an adapter-generated shell invocation to source the
validated environment script and emit a framed, allowlisted environment for a
child process. It reports an interactive-required result without attempting to
answer prompts. It also owns exact non-shell Git clone/checkout vectors and
their cancellation. The embedded-shell backend receives the validated context
and owns any interactive exchange. Shell environment changes stay inside its
child process.

The CLI executes typed onboarding effects, keeps the terminal responsive, and
constructs the managed backend from the correlated captured child environment.
It verifies the backend by requesting a typed workspace snapshot before
installing it as the active session. CLI `--build-dir`/`--backend` values are
explicit overrides for automation and diagnostics; an omitted build directory
creates an unconfigured session instead of treating the current directory as
a build.

## Persistent daemon and attachable client architecture

Yoctui remains one Rust-native terminal product. The installed package may
expose daemon and client modes as subcommands of the `yoctui` executable; it
does not introduce Electron, a browser runtime, or a network service. The
normal end state is an attachable Ratatui client connected to one per-user
daemon on the build host. An explicit standalone mode remains available for
debugging and minimal environments while migration is in progress. Standalone
mode uses the same model, typed effects, adapters, and safety policies; it is
never a second ad-hoc execution implementation and it does not promise that
work survives client exit.

### Process and crate responsibilities

The daemon is the sole authority for state and execution that must outlive a
client connection. It owns:

- the selected workspace/project identity and loaded optional project profile
- the BitBake controller, connection, capabilities, metadata snapshots, and
  reconnect/restart lifecycle
- every background-job coordinator and retained bounded history, log, error,
  QEMU, Wic, SDK, testing, QA, security, maintenance, and utility result
- PTY masters, terminal-emulator state, process groups, scrollback, and session
  metadata
- global sequence allocation, client subscriptions, request arbitration,
  confirmation leases, persistence, recovery classification, and resource
  limits

The client owns terminal initialization/restoration, crossterm input, Ratatui
rendering, responsive focus, open dialogs and editors, command-palette state,
pane layout, scroll position, local selections, and prefix/mouse interaction.
It sends typed intent and renders daemon snapshots/events. It never becomes a
parent or lifetime owner of a BitBake process, background job, or persistent
PTY. Client-local presentation state is not broadcast merely because two
clients view the same daemon.

Existing dependency direction remains binding. `yoctui-model` contains pure
global and client-local domain types; `yoctui-protocol` contains stable wire
types and framing; `yoctui-bitbake` contains BitBake/process/PTY adapters;
`yoctui-app` maps typed input, daemon commands, events, and effects; the UI
renders replicas; CLI modes compose these pieces. A daemon runtime support
crate may sit beside the CLI, but it may depend only in that direction and may
not move UI concerns into backend adapters. Existing typed actions, effects,
job coordinators, correlations, and confirmation policies are migrated into
the daemon boundary rather than duplicated.

### Daemon-owned BitBake controller

`yoctui-bitbake::BitBakeServerController` is the UI-independent lifecycle
authority used by the daemon. It validates the absolute source/build/init
context once, then delegates supported interface details to a typed async
`BitBakeServerAdapter`; it never accepts a shell command string. Detection,
start, connect, disconnect, stop, restart, and reconnect move through explicit
`Unknown`, `Detecting`, `Unavailable`, `Available`, transitional, `Connected`,
`Recovering`, and `Failed` states with a checked generation and diagnostic.
Every adapter operation has a configured nonzero timeout.

Observations bind a validated endpoint, bounded server identity, optional
version, and deduplicated typed capabilities. A connected session must name the
same server and supplies a bounded connection identity. Restart disconnects
the owned session, stops the observed server, starts a new observation, and
reconnects only when the controller was connected before; reconnect replaces
only the session. The abstraction does not yet claim a live BitBake transport:
supported socket behavior is implemented by `BITBAKE-SOCKET-001`, while
shell-free previewable CLI gaps belong to `BITBAKE-CLI-CONTROL-001`.

`BitBakeSocketAdapter` supplies the Unix implementation without reimplementing
BitBake's Python pickle or file-descriptor-passing protocol in ad-hoc Rust.
The daemon launches the existing bounded NDJSON bridge in the selected build
environment; the bridge imports that workspace's own `bb.tinfoil`, whose
official process-server connector passes the event, command, and reply file
descriptors over `bitbake.sock`. Rust validates the configured absolute bridge
path, build identity, non-symlink same-UID socket (or explicit `BBSERVER`
identity), version, capabilities, and server inode identity before adopting
the transport. Bridge children are kill-on-drop, messages remain bounded, and
responses must carry one of the bounded known command correlations.

Detection proves usability by completing a Tinfoil connection and authoritative
workspace inspection rather than treating socket existence as connection
proof. Start allows Tinfoil to use BitBake's supported start-or-reconnect path;
connect transfers that proven transport into the controller session; disconnect
runs Tinfoil `clientComplete`; stop sends the process-server `terminateServer`
operation used by BitBake's own kill-server path and waits for socket removal.
Unexpected EOF, malformed socket identity, wrong correlation, timeout, and
adapter errors become typed controller failure. This does not make the Python
bridge a second product or UI: BitBake itself is Python, while all Yoctui
ownership, lifecycle, validation, state, and user interaction remain Rust.

The socket adapter remains the primary server-control path. Where a supported
socket operation is unavailable, `BitBakeCliCommand` exposes only the typed
status, server-start, and server-stop fallbacks corresponding to BitBake's
`--status-only`, `--server-only`, and `--kill-server` options. Capability flags
must authorize the operation before an immutable command is constructed. Its
preview contains the exact executable, single argument, working directory,
deadline, and per-stream bound; execution uses `tokio::process::Command`
directly with the captured build environment and no shell. The runner drains
but retains only bounded output, owns a child process group, applies a deadline,
and supports TERM-then-KILL cancellation. Typed success, nonzero, timeout, and
cancellation outcomes prevent UI code from parsing process output or status.

Controlled restart is daemon orchestration above that controller. The pure
model derives a bounded affected-work list from the authoritative primary build
and background jobs, and binds confirmation to the server identity, controller
generation, and exact job identities. Immediately before execution the
coordinator reconstructs that preview from current work; changed server state
or jobs make it stale. With active work it refuses missing or mismatched
confirmation. An accepted operation uses the controller's bounded
disconnect/stop/start/reconnect sequence and then invokes a typed authoritative
metadata refresh. Job state, histories, client replicas, and presentation state
are inputs or consumers rather than being replaced by restart orchestration.

### State partitions

State has three explicit partitions:

| Partition | Owner | Examples | Persistence |
|---|---|---|---|
| Global live | daemon | BitBake connection, active jobs, PTYs, global metadata and logs | metadata only; live handles are never serialized |
| Durable safe | daemon | workspace identity, bounded history/logs, session names/kinds, profile identity, recovery records | atomic versioned files under the user state directory |
| Presentation | each client | focus, dialogs, pane tree, scroll offsets, terminal dimensions, prefix state | user-local client preferences only |

A persisted process ID is evidence about a former process, never proof that it
is still owned or running. Secrets, captured build environments, PTY contents
unless explicitly enabled within bounds, open file descriptors, socket
credentials, confirmation leases, and arbitrary child environment values are
not durable state. Layout records refer to stable session identities but remain
client-local; unavailable sessions collapse safely on restoration.

`yoctui-model::DaemonGlobalState` is the pure first implementation of this
partition. It owns daemon instance/sequence/generation revision, validated
collection limits, authoritative workspace, build-environment, project-profile,
BitBake lifecycle/capabilities, boot/recovery session metadata, and bounded
global log/error/task-history placeholders. Every global mutation trims its
collections and advances sequence and generation with checked arithmetic.
`ClientDaemonReplica` moves explicitly through disconnected, synchronizing,
current, and stale states and replaces its entire snapshot safely.
`ClientPresentationState` separately owns screen, focus, Navigator selection,
theme, and pane-layout revision; changing it cannot mutate global authority.
Job-family fields and execution ownership enter only through the following
registered state-migration tasks.

`DaemonJobState` now reuses the existing typed build/background-job, active and
completed task, bounded `LogState`, signature, package, image, SDK, testing,
security, QA, maintenance, QEMU, and Wic state instead of defining parallel
summaries or coordinators. It also provides the typed placeholder list for
daemon PTY session metadata. Capture and replica installation copy only these
long-lived workflow fields; client screen, focus, dialog/editor drafts,
searches, selections, and notifications remain untouched. The app boundary
exposes mechanical capture/install functions so later protocol/runtime work
does not parse or reinterpret workflow state.

The foreground daemon constructs `DaemonGlobalState` from its authenticated
runtime instance and boot metadata, installs initial typed job state through
`DaemonStateAction` and the checked daemon reducer, and retains that authority
outside every client connection. After a successful protocol handshake,
`Attach` returns a typed protocol snapshot derived by the app boundary;
`Detach` closes only that client attachment. Reattachment reads the same
daemon-owned state and revision. The current runtime snapshot is deliberately
an attach baseline, not the gap-free incremental synchronization promised by
`DAEMON-SNAPSHOT-001`, and it does not move process runners into clients.

### Local IPC and instance identity

Unix uses a Unix-domain socket. The deterministic default is
`$XDG_RUNTIME_DIR/yoctui/daemon.sock`; when `XDG_RUNTIME_DIR` is absent Yoctui
may use the verified per-user `/run/user/<uid>` directory, but it fails with a
clear diagnostic rather than falling back to a shared `/tmp` path. The runtime
directory is owned by the effective user with mode `0700`; the socket is mode
`0600`. Creation and stale cleanup use no-follow, ownership, type, and peer-UID
checks. No TCP listener exists by default.

`yoctui-protocol::daemon_ipc` implements this boundary without exposing a TCP
transport. It canonicalizes and verifies the private runtime root, creates the
`yoctui` directory with mode `0700`, binds `daemon.sock` with mode `0600`, and
authenticates Linux peers with `SO_PEERCRED`. Unix targets without an
implemented native peer-credential API fail closed. Stale cleanup rejects
symlinks and non-sockets, removes only a same-UID socket that refuses a local
connection, and refuses a socket that accepts. Listener cleanup records the
bound device/inode and removes only that exact socket. Client connect retries
only expected unavailable states until its deadline; reads/writes apply
deadlines and reject oversized length prefixes before allocating payloads.

Each daemon start creates an unpredictable `DaemonInstanceId`, start time, and
boot identity. Clients also receive stable-for-one-connection identities. The
typed protocol uses bounded length-delimited frames, bounded queues, deadlines,
and a versioned handshake with capability negotiation. Client requests carry
correlation IDs; daemon events carry a monotonically increasing instance-local
sequence. Unsupported major versions fail closed with actionable diagnostics.
Minor evolution is additive: unknown optional capabilities and events may be
ignored only when the negotiated version says that is safe. A daemon instance
change always invalidates outstanding requests, writer leases, and incremental
sequence assumptions.

The daemon wire protocol is separate from the Python bridge's NDJSON protocol.
It uses a four-byte big-endian length followed by one JSON payload, capped at
4 MiB before allocation or decoding. `ClientMessage` and `ServerMessage` are
closed typed envelopes. The handshake exchanges supported version ranges,
client/daemon/boot identities, capabilities, and concrete limits. Attach and
resume carry a daemon instance plus last sequence; snapshots carry both their
sequence watermark and state generation; incremental events carry the next
ordered sequence and resulting generation. Commands carry a request ID and
the generation reviewed by the client, while results return the same request
ID. Ping/Pong deadlines expose stale clients. Resync is an explicit server
message rather than an inferred timeout.

Daemon messages include typed BitBake lifecycle, job families, profile state,
PTY summaries/screens/raw output, client presence, recovery warnings, and
bounded logs. Client messages include subscriptions, typed build/lifecycle and
PTY commands, writer-epoch-guarded input/resize, pane-to-session attachment,
server-relevant terminal mouse input, detach, and heartbeat response. Unknown
major versions fail. Within a negotiated compatible minor version, unknown
optional capabilities and incremental daemon events decode as `Unknown` and
are ignored; unknown commands and required snapshot fields are errors.

### Attach, detach, and synchronization

Attach authenticates the local peer, negotiates protocol/capabilities, then
atomically installs a subscription at a sequence watermark. The daemon sends a
bounded consistent snapshot for that watermark followed by ordered events
strictly after it, so there is no snapshot/subscription gap. A client that has
a retained sequence may request replay; if history is missing, the daemon
explicitly replaces the stale replica with a new snapshot. Until synchronization
completes the client shows reconnecting state and does not issue consequential
commands against stale data.

`DaemonSnapshotJournal` implements that single-owner cut with validated limits
for snapshot bytes, retained event count, recent logs, and individual framed
events. Publishing checks sequence and generation overflow, applies the event
to a candidate snapshot, verifies its bound, and only then atomically commits
both snapshot and retained event. A same-instance cursor within retention gets
only strictly later ordered events; a missing, wrong-instance, expired, or
future cursor receives an explicit replacement reason and the current bounded
snapshot. `DaemonClientSnapshot` rejects any sequence/generation gap, marks its
replica stale, withholds its resume cursor, and becomes current only after a
safe full replacement. The foreground runtime uses the same journal for fresh
attach and resume rather than maintaining separate socket-specific state.

Detach closes only the client subscription and releases its ephemeral focus,
confirmation, and PTY-writer leases. It does not cancel jobs, disconnect
BitBake, close PTYs, or shut down the daemon. EOF, terminal close, client crash,
SSH loss, and ordinary `q` follow the same detach path. Reattachment restores
daemon-global state and terminal-emulator screens; client-local layout is
restored separately and only for still-valid session IDs.

### Lifecycle and shutdown

`yoctui` normally connects to the local daemon and may auto-start it according
to an explicit user setting. `yoctui daemon foreground` is the debuggable
non-daemonizing service form. Start, status, stop, and restart use Rust process
and service-manager APIs, PID/runtime records, and the typed control protocol;
they do not use shell backgrounding tricks.

Stopping or restarting the daemon is distinct from detaching a client. With
active jobs or PTYs, a normal stop/restart is refused until the UI/CLI displays
the affected identities and receives the usual explicit confirmation. An
approved graceful stop stops accepting work, flushes safe state, asks owned
jobs and process groups to terminate, applies bounded graceful then forced
cleanup, disconnects BitBake, acknowledges clients, and removes its socket.
Force remains explicit and reports what may become `Lost`. Systemd user-service
integration is preferred where available and requires no root; an unavailable
user manager produces a documented direct-process fallback rather than a
false success.

The first lifecycle slice is implemented as `yoctui daemon
start|status|stop|restart|foreground`. Direct start resolves the current Rust
executable and starts its foreground subcommand in a new process group with
closed standard streams; it does not construct a shell command or double-fork.
It waits for an authenticated typed handshake before reporting success.
Foreground mode binds IPC, replaces only a provably stale prior runtime record,
writes an atomic private `daemon.json` containing PID, random instance ID,
boot ID, executable identity, and start time, and handles graceful protocol
shutdown plus SIGTERM cleanup. Status validates boot identity, PID liveness,
`/proc/<pid>/exe`, live socket handshake, and matching daemon instance. Stop
uses a typed request and waits for socket cleanup; restart composes verified
stop and start. Interactive automatic attach/optional auto-start will call
these same lifecycle APIs when `CLIENT-ARCH-001` moves the client boundary;
the current single-process TUI is not falsely reported as attached meanwhile.

Optional service-manager integration generates
`$XDG_CONFIG_HOME/systemd/user/yoctui.service` (or the corresponding
`~/.config` path) with the canonical installed executable and `daemon
foreground` argv. The unit is a simple unprivileged user service with
`Restart=on-failure` and `NoNewPrivileges=true`. Generation rejects control
characters and unsafe existing service-file types and atomically writes a
private regular file. `yoctui daemon service
install|uninstall|start|stop|restart|status` invokes only shell-free
`systemctl --user` vectors. Install reloads the user manager and prints the
explicit `enable --now` auto-start command. A missing or failed user manager
returns the exact direct-process `yoctui daemon start` fallback; no command
requests root access or silently claims service activation.

Version 1 daemon persistence lives in the user state directory
(`$XDG_STATE_HOME/yoctui/daemon-state.json`, with the standard user-state
fallback when needed), not the reboot-volatile runtime socket directory. The
directory is private and the atomically replaced, fsynced file is mode `0600`,
same-UID, non-symlink, regular, and bounded to 4 MiB. Its typed schema contains
only prior daemon/boot/reconnect watermarks, workspace and profile summaries,
BitBake version/capabilities, job history, terminal name/kind/cwd/dimensions and
prior lifecycle, optional bounded logs, recovery warnings, layout revision and
session-name metadata, and user preferences. It has no PID, process group,
writer lease, or client-presence field. Every terminal entry carries a required
false `live_process_persisted` invariant, which both reads and writes reject if
violated. Runtime startup and clean shutdown write this safe checkpoint;
classification and reconstruction belong to `DAEMON-RECOVERY-001`.

### Crash, daemon restart, and host reboot

On daemon crash, connected clients detect EOF/timeout and enter disconnected
state. They may retry with bounded backoff and must handshake again. Recovery
loads only validated durable state and classifies every former operation:

- a supported external BitBake server may be detected and reconnected, after
  identity/capability validation
- a job with no provable live owner becomes `Lost`, never `Running`
- a PTY whose master/emulator ownership was lost becomes `Lost` or `Exited`;
  the daemon does not attach to an arbitrary matching PID
- persisted history, names, layouts, and bounded logs may be restored as
  historical records
- explicitly restartable workflows may offer a reviewed relaunch, never an
  automatic arbitrary command replay

A host reboot is a stronger boundary. The daemon may auto-start after login or
boot through supported user-service configuration, but arbitrary child
processes and PTYs did not survive. Boot identity mismatch marks former live
records `Lost`/`Stopped`, lists them honestly, and offers relaunch only for a
typed restartable workflow. No PID reuse check can upgrade such a record to
running. Jobs survive an SSH/network disconnect because the daemon is local to
the build host; survival through logout depends on the configured user service
and is reported explicitly. Nothing here claims process survival across an
actual host reboot.

Restart recovery now validates the durable checkpoint before constructing the
new daemon instance and event sequence. It restores workspace/profile summary,
bounded logs, job history, and terminal name/kind/cwd/dimensions, but clears
all client presence, viewers, and writer leases. Every job or PTY whose prior
lifecycle was nonterminal becomes `Lost`; terminal history remains terminal.
A persisted BitBake version or capability set is restored only as disconnected
identity with a typed reconnect recommendation, never as proof of a live
server. A profile summary does not reconstruct profile contents: the model
returns to `NotLoaded` and requires the normal bounded validation path. Same-
boot daemon restart and changed-boot recovery produce distinct visible
warnings. The later BitBake controller may attempt a supported reconnect; no
external process is restored merely by PID or command resemblance.

For host reboot, installing the generated user unit is not itself an implicit
enablement: the user explicitly runs the printed `systemctl --user enable
--now yoctui.service` command. Once enabled, `WantedBy=default.target` starts
the unprivileged foreground daemon with the user manager after login (or under
the host's configured user-lingering policy); `Restart=on-failure` covers
daemon failure, not host power loss. Changed boot identity is always reported
as `HostReboot`. Persisted sessions remain listed as `Lost`, and only entries
already marked restartable yield a typed relaunch intent containing prior
session ID, name, kind, cwd, and dimensions. No argv, environment, PID, or
process group is persisted or automatically executed. A future session action
must reconstruct and confirm a supported typed workflow before relaunch.

### Multi-client arbitration

Multiple authenticated clients may observe the same global state and receive
the same ordered events. Global commands are serialized through the daemon
reducer. Commands include the authoritative state generation they were
reviewed against; stale or conflicting commands are rejected with a typed
reason and refreshed context. Destructive confirmations use short-lived,
single-use daemon-issued leases bound to client, request, preview hash, and
state generation, so one client cannot confirm another client's preview.

Focus, dialogs, selections, layout, mouse hover/drag, and terminal dimensions
remain client-local. PTYs allow many viewers but exactly one active writer.
Taking control is explicit, the writer identity is visible, and disconnect or
lease timeout releases control. Input from competing clients is never silently
interleaved. Resize policy is explicit: the writer controls PTY dimensions;
viewers render/crop the authoritative terminal state without fighting the
size. Daemon-global operations such as BitBake restart additionally report all
affected clients/jobs before confirmation.

### Daemon-owned PTY session contract

The daemon owns each PTY master, child and process group, terminal-emulator
state, authoritative dimensions, bounded scrollback, exit status, and session
metadata. The interactive client never inherits or proxies the master file
descriptor. Closing, crashing, or detaching the last client removes a viewer;
it does not signal the child. Each session has a daemon-allocated stable,
non-reused ID plus a validated bounded display name and typed kind (build/source
shell, layer/recipe/devtool context, SDK/native shell, devshell, menuconfig, or
another registered interactive workflow). Command identity is a typed workflow
and exact executable/argument identity, not an arbitrary project-profile shell
string.

The daemon opens a Unix PTY, creates a new owned child process group/session,
sets the slave as controlling terminal, applies the initial window size and
terminal modes, and closes unrelated descriptors. Working directories must be
absolute, canonical, non-symlink-resolved typed workspace locations authorized
by the workflow. Environment comes from the already verified build or SDK
environment plus a small explicit terminal allowlist/override; project profile
loading never sources files or executes hooks. Secrets and full environments
are neither broadcast nor persisted. Child PID/process-group identity is live
daemon state only.

PTY transport uses versioned typed commands and events carrying stable session
ID, monotonically ordered output sequence, writer lease epoch, and bounded byte
chunks. Input and output are terminal bytes, not presumed UTF-8; the emulator
retains decoded cells and replacement behavior while raw bytes remain bounded
and available only as needed for terminal fidelity. IPC frame, chunk, queue,
scrollback, session-count, and terminal-dimension limits apply before
allocation. Backpressure produces explicit dropped-output/refresh-required
metadata; it never creates an unbounded queue. A client recovering from an
event gap requests a consistent emulator screen plus bounded scrollback
snapshot and resumes after its watermark.

Resize is a typed, range-checked command. Only the current writer may change
the authoritative rows and columns; the daemon applies `TIOCSWINSZ` to the PTY
and signals the foreground process group as the platform requires. Viewers crop
or letterbox locally and do not race dimensions. Attaching adds a viewer and
returns the current emulator/screen, scrollback bounds, lifecycle, dimensions,
and writer identity. Detaching removes that viewer and releases its writer
lease without closing the session. Client EOF, SSH loss, or normal UI exit has
the same detach semantics. Reattach to a running or exited session restores its
current or final terminal state; recovered metadata whose process cannot be
proved becomes `Lost`, never `Running`.

Many clients may view one session, but exactly one may write. Control is
acquired explicitly against the current epoch, exposes the writer client
identity to all viewers, and is released on explicit relinquish, disconnect,
lease timeout, session exit, or daemon restart. Stale-epoch input and resize are
rejected. Input from two clients is never merged. A prefix command is consumed
client-side before PTY input so detach, pane navigation, help, and control
actions cannot leak into the terminal application; prefix timeout and literal
prefix forwarding are defined by the later keyboard task.

Normal close sends the typed workflow's graceful termination signal to the
owned foreground/process group, waits a configured bounded interval, then
requires a separate forced-termination action or explicit policy before
`SIGKILL`. Session kill and other destructive operations use Yoctui preview and
confirmation rules. Child exit closes the slave, drains bounded remaining
output, records exit/signal status, freezes the final emulator state, releases
control, and reaps every owned child. Daemon shutdown applies its explicit
session policy; daemon crash recovery marks an unrecoverable PTY `Lost`.

Scrollback is daemon-owned and bounded by lines, cells, and bytes. Search query,
match selection, viewport, and copy-mode cursor are client-local; the daemon
serves bounded text/cell ranges from the session snapshot and never writes the
host clipboard. Copy uses the existing client clipboard effect. Paste is
accepted only from the current writer, is byte- and rate-bounded, and is sent
as literal terminal input without shell interpretation. When the emulator says
bracketed paste is enabled, the daemon wraps the payload with the standard
terminal markers; otherwise it sends the literal payload. NUL/control bytes
outside explicitly supported terminal input and oversized payloads are
rejected. Profiles cannot provide automatic paste or startup keystrokes.

`yoctui-bitbake::PtyRunner` is the Unix execution adapter beneath that model.
It opens a real master/slave PTY, creates a child session with the slave as its
controlling terminal, assigns the session leader as the owned process group,
and executes only the validated exact command/cwd and captured environment.
The master is split into daemon-owned asynchronous reader/writer handles; raw
bytes, including invalid UTF-8, cross fixed-size chunks through a bounded queue.
Writer-epoch checks guard bounded input, and resize first validates a model
transition before applying `TIOCSWINSZ`. Natural exit reports the real code or
signal and clears live ownership. Termination signals the owned group, waits a
bounded grace interval, escalates to `SIGKILL`, reaps the child, and reports the
actual exit identity. Dropping a live runner kills only its recorded group and
child. The adapter contains no terminal rendering or UI state.

Terminal interpretation uses the maintained Rust `vt100` parser/emulator behind
`yoctui-model::TerminalEmulator`; Yoctui does not parse ANSI/VT sequences ad
hoc. The wrapper accepts bounded raw-byte feeds, owns configured bounded
scrollback, validates resize and maximum screen cells, and exports Yoctui-owned
typed cells, colors, styles, cursor, alternate-screen, application cursor/keypad,
bracketed-paste, and mouse modes. Snapshots clamp requested scrollback offsets,
restore the live viewport after inspection, and contain no crate-specific types
across the model boundary. Default callbacks deliberately do not execute OSC
clipboard requests or other external side effects. Shell/editor/ncurses-style
fixtures cover cursor addressing, Unicode box drawing, SGR, alternate screen,
scrollback, resize, paste, keypad, cursor, and mouse modes.

`DaemonPtySession` composes one runner and emulator for attach semantics while
the bounded multi-session registry remains a later task. The daemon pump feeds
every output event into the emulator independently of viewer presence. Attach
adds the typed client viewer and returns the current listing plus bounded screen
snapshot; reattach therefore does not replay output through a client. Detach,
prefix return, client EOF, and SSH loss all use the same model detach transition,
which also releases an owned writer epoch but never signals the runner. Input
and resize remain writer-epoch checked, and resize updates both PTY and emulator.
Natural exit freezes the final emulator and listing; runner/model loss maps to a
distinct `Lost` listing. The current coordinator is staged in the CLI daemon
composition for the subsequent multi-session/runtime protocol tasks and is not
a client-owned lifetime wrapper.

PTY context launch resolution lives in `yoctui-app`. A
`PtyContextAuthority` is constructed only from the verified build environment
and authoritative workspace/layer/recipe/Devtool/deploy/SDK inventories. It
canonicalizes directories and executable shell identity, validates bounded
captured environments, rejects duplicate or stale typed identities, and
revalidates both path and executable at launch. Actions select an identity—not
an editable path or command—and produce an exact `PtyCommandIdentity`, cwd,
environment identity/data, kind, and workspace context accepted by the PTY
model. External configured layers, Devtool workspaces, deploy roots, and SDK
roots become explicit authorized context roots. Project-profile command text is
not an input to this path and cannot cause shell execution.

Interactive Devtool routing is a narrow layer above that catalog. It accepts an
authoritative `DevtoolStatus` and supports only the selected Devtool workspace
shell or exact `devtool edit-recipe <validated-recipe>` PTY preview. Workspace
identity and canonical source must still match at preview time, and the
executable is a verified regular executable. Modify, update-recipe, finish,
deploy and reset explicitly return `UseBackgroundJob`, preserving the existing
typed `DevtoolCommandSpec`/job coordinator rather than duplicating long-running
execution inside a terminal.

Menuconfig/devshell routing likewise consumes an authoritative bounded recipe
and task catalog. A closed Rust enum permits only `menuconfig`, `devshell`,
`nconfig`, and `xconfig`; kernel and U-Boot shortcuts resolve the current typed
provider identities rather than assuming recipe names. The preview verifies the
exact recipe file identity and advertised task, then emits only
`bitbake -c <task> <recipe>` with the verified executable and captured build
environment. There is no free-form task/argument field and the client remains
attached to Yoctui while the daemon-owned PTY runs the terminal application.

SDK shell initialization is split across the adapter and typed context
authority. `SdkShellAdapter` inspects one canonical, direct-child
`environment-setup-*` file under an explicitly selected SDK root, records its
bounded content digest, and revalidates that identity before capture. Capture
uses an argv-separated constant Bash program in an isolated child with a
minimal inherited environment; it never mutates the daemon or client process.
NUL-delimited output, variable count/value/total bytes, names, timeout, and
dangerous shell-control variables are bounded or rejected. The resulting
environment and canonical interactive shell become a verified SDK context.
The app router previews either that installed SDK context as `SdkShell`, or
the already verified build context as `NativeShell`; both launch only the
exact interactive shell argv through the daemon PTY runner. Project profiles
cannot name an environment setup script or inject capture commands.

The attachable client transport is a typed session layered directly on the
authenticated local IPC connection. It validates nonzero client/daemon
identities, exact protocol compatibility, unique negotiated capabilities, and
daemon-advertised resource limits before attach. Attach accepts the replica's
resume cursor, collects ordered replay messages, verifies the final snapshot
instance and watermark, and exposes typed snapshot/event/command-result/
resynchronization messages to the client runtime. Subscribe, unsubscribe and
command requests use only protocol types. Ping handling is internal, explicit
detach waits for daemon acknowledgement, and reconnect creates a fresh secure
connection while retaining client identity; it never owns jobs or PTYs.

Replica installation converts protocol-owned snapshots into a protocol-free
`ClientDaemonView` in `yoctui-model`. The view carries revision, daemon
identity, BitBake lifecycle, bounded job/PTY summaries, client count, logs and
recovery warnings. `yoctui-app` owns the mapping and applies only gap-checked
protocol events before replacing the derived view. Ratatui renders this typed
state and does not inspect wire messages. Screen, focus, Navigator selection,
theme, dialogs, editor state, layout and other presentation fields are never
part of replica replacement; disconnect changes connection status without
discarding the last honest daemon snapshot.

The first interactive runtime slice attaches before terminal work begins,
installs and nonblocking-polls the replica during every event-loop iteration,
and explicitly detaches at shutdown. Ordinary `Effect::Start` and
`Effect::Cancel` requests are translated—not re-parsed—into correlated,
generation-checked daemon build commands when attached. Local execution remains
only as an explicitly reported compatibility path while standalone policy is
pending. Moving the other existing typed job families and their runner
ownership is a separate required child gate; attach wiring alone is not proof
that those jobs survive client exit.

Devtool is the first migrated job family. The protocol uses a closed
`DaemonDevtoolOperation` enum with typed recipe/target/destination fields and a
canonical build-directory identity. The client converts existing Devtool
effects into that wire type and never sends argv or shell text. The daemon
revalidates the build directory and operation, builds the existing
`DevtoolCommandSpec`, and owns `DevtoolJobRunner`, its process group, bounded
output, cancellation channel and terminal event. Job/log changes enter the
same sequenced daemon journal. The supervisor outlives any client connection;
detaching a client neither drops nor cancels the runner.

SDK jobs use the same ownership boundary with a separate closed protocol. The
wire operation distinguishes typed publication and native-tool requests,
including exact artifact identity, setup root, executable, bounded arguments,
and selected SDK context roots. The daemon reconstructs the existing
`SdkPublishPreview` or `SdkNativePreview`, validates it through
`SdkToolAdapter`, and owns `SdkToolJobRunner`, output, timeout, cancellation and
terminal state. Client disconnect has no effect on the runner; output is
converted into bounded sequenced daemon logs/job events.

### Security and trust

The daemon runs as the invoking user and never escalates privilege. Local-only
access, runtime-directory ownership, socket mode, peer credentials where
available, bounded decoding, command allowlists, correlation validation,
queue/resource limits, and environment filtering form the IPC boundary. Paths
are canonicalized and checked against their typed workspace/context before
use. PTY process groups are daemon-owned and signals target only recorded
owned groups.

Project profiles remain inert team intent. Connecting a persistent daemon does
not grant a profile permission to source scripts, run hooks, create PTYs, or
relaunch persisted commands. Environment initialization, clone, build,
destructive maintenance, daemon/BitBake restart, and PTY creation still cross
their existing typed preview/confirmation/capability boundaries. Client input,
including mouse events, becomes a server command only when it has daemon-global
meaning; raw terminal mouse/input bytes are accepted only from the current PTY
writer and within negotiated bounds.

### SSH and standalone interaction

Remote use means SSH into the build host and run the normal client there. The
client connects to that host's local Unix socket. No unauthenticated TCP proxy
or remote browser service is added. Dropping SSH detaches that client; daemon
jobs and PTYs continue subject to the service/lifecycle guarantees above, and
the next SSH session performs normal authenticated reattachment.

During migration, `--standalone` is an explicit diagnostic/minimal mode and the
current single-process implementation supplies it. Default behavior changes to
daemon attach only after client/daemon parity tests pass. Thereafter standalone
remains tested but clearly reports that client exit owns and stops its work.
Daemon and standalone modes share persisted user preferences but use separate
runtime/process state and cannot simultaneously own the same workspace's
BitBake controller. The second owner request is rejected rather than racing.

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
### Daemon-owned QEMU jobs

QEMU launch requests use closed typed identities. The client sends the
validated image, executable, launch modes, bounded arguments and build
directory; the daemon reconstructs the command through `QemuCommandSpec`, owns
the process group, and publishes output, cancellation and terminal lifecycle
events.

Wic image creation uses the same daemon-owned runner boundary. Device writes
remain a separate destructive operation so exact discovered device identity
can be revalidated before execution.

Selftest sessions also use daemon-owned typed requests and the existing
validated test runner; managed BitBake test builds remain on the BitBake job
controller path until their dedicated migration gate.
