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
correlated lifecycle state. It may reuse
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
outcomes. It may reuse `BuildRequest`, `RecipeIdentity`, `Layer`, and the shared
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
process-group runner as sstate operations.

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
