# Compatibility and Validation Evidence

Yoctui requires stable Rust for compilation and an initialized Yocto
environment for production BitBake control. This document records observed
evidence, not compatibility inferred from version numbers. The authoritative
metadata and build result always come from the selected BitBake environment.

## Normative dynamic compatibility contract

**Yoctui functionality is Yocto-feature-correlated.** The installed Yoctui
binary defines the operations and adapters Yoctui knows about. The connected
Yocto/OpenEmbedded/BitBake environment supplies the evidence that determines
which operation and implementation is safe now. Binary support alone never
makes a feature available.

### Environment identity and authority

An environment identity contains only detected authoritative values. Every
field is independently optional and preserves `Unknown`; values are never
synthesized merely to produce a friendly release label.

| Identity field | Authoritative source |
|---|---|
| Canonical build directory and source roots | Selected initialized environment plus canonical filesystem identity |
| BitBake version and executable | Imported `bb.__version__`, negotiated backend response, or bounded `bitbake --version` probe tied to the selected environment |
| OE-Core/Poky release name and version | Authoritative metadata variables or release files from a configured OE-Core/Poky layer |
| `DISTRO` and `MACHINE` | Effective BitBake datastore values for the selected build |
| Configured layers and series compatibility | BitBake layer inventory and each configured layer's authoritative compatibility metadata |
| Available tooling | Canonical executable identity discovered in the initialized environment, followed by command/option probes where required |
| Backend and protocol version | Negotiated daemon/bridge handshake |

Repository branch names, directory names, host distribution labels, arbitrary
PATH entries, nearest Git tags, and numeric resemblance are weak heuristics and
must not invent a Yocto release. They may be retained as diagnostic hints only,
never as authoritative identity or capability evidence. A mixed-layer build is
identified as the configured build it actually is; it is not forced into a
single Poky release family.

An exact environment fingerprint covers canonical workspace/build/source
identity, BitBake executable identity and version, initialized environment
identity, configured layer roots/revisions/series, and relevant configuration
files. Capability snapshots and probe caches are scoped to that fingerprint.

### Detection and evidence precedence

Capability decisions use this order, strongest first:

1. A safe direct positive or negative probe against the selected environment.
2. A negotiated backend, protocol, API, metadata, or native-event capability.
3. Authoritative configured metadata proving required task/class/variable support.
4. A centralized release/version fallback rule when no practical direct probe exists.
5. `Unknown` when evidence is absent, partial, stale, contradictory, or a probe fails.

Direct detection is preferred because commands, options, tools, classes, and
APIs can vary independently of a release label. A positive probe may override a
conservative version fallback when the catalog marks that override safe. A
negative authoritative probe overrides an optimistic fallback. Conflicting
strong evidence yields `Unknown` with the conflict recorded; it is never
resolved in favor of availability merely to keep an action enabled.

Probes are non-mutating, shell-free argv processes or typed API negotiations.
They have deadlines, bounded stdout/stderr, bounded result counts, process-group
cancellation, and sanitized evidence. Allowed examples include executable
identity, `--version`, `--help`, supported-subcommand/option inspection,
read-only metadata variables, and protocol handshakes. A probe may not start a
build, mutate metadata, create a workspace, contact a target, write an image,
or otherwise exercise the consequential operation it is trying to authorize.

### Release families and support policy

Release families are evidence-policy groupings, not renderer logic and not a
substitute for capability probes:

- **Supported maintained family:** an official release series inside the
  declared support window with current required live evidence.
- **Supported older family:** the minimum claimed series, validated to preserve
  baseline workflows and exercise at least one materially older adapter or argv.
- **Current stable/latest tested family:** the newest official stable series
  for which the repository has current required live evidence.
- **Development/future family:** an official development snapshot or unknown
  future version; positively probed behavior may run, but historical behavior
  is not presumed.
- **Unsupported family:** an environment outside policy or one lacking a safe
  required baseline after probing; its diagnostic workspace may still open.
- **Mixed/unknown family:** release correlation is absent or ambiguous; direct
  probes alone decide individual behavior.

The support window is evidence-driven and recorded in
`compatibility-matrix.md`; it is not compiled into UI conditionals. Current
non-fixture anchors are exact live-validated revisions: maintained Scarthgap 5.0.19
with BitBake 2.8.1 at the proposed lower boundary, and Wrynose 6.0.2 with
BitBake 2.18.0 as the latest published stable observed on 2026-08-19. Both live
records satisfy the machine-readable evidence policy and bound the current
**Claimed supported** window. The claim attaches to the exact recorded release
identities and required workflow set; it does not replace per-capability
probing or silently extend to another point revision. An earlier BitBake 2.19.0
development snapshot remains merely Partially tested.

An unsupported release does not crash the application. Yoctui opens in
diagnostic degraded mode, identifies what it can, enables safe positively
verified operations, and disables the rest with exact reasons. An older
supported release retains every safely functioning capability and uses a
maintained fallback where one is cataloged. A future unknown release is not
rejected by name/version; it exposes only positively detected behavior and
leaves uncertain behavior `Unknown`.

Snapshots expose an operating mode derived from their mixed states, never from
a release-name allowlist. `Full` means every recorded behavior is directly
available. `Degraded` means at least one available/limited behavior remains
while another is limited, unavailable, unknown, or unsupported. `Diagnostic`
means no action capability is currently enabled, but identity, compatibility,
Doctor, settings, and safe navigation remain usable. No mode terminates the
application merely because one workflow is absent.

### Capability states

Every catalog capability has exactly one runtime state plus bounded reason and
evidence records:

| State | Meaning and UI behavior |
|---|---|
| `Available` | Required behavior and preferred implementation are positively evidenced; the action may be enabled. |
| `AvailableWithLimitations` | A safe implementation exists but has named constraints or uses a maintained fallback; the action is enabled with limitations explained before use. |
| `Unavailable` | Current authoritative environment evidence shows a required tool, command, option, metadata feature, artifact, or configuration is absent; action is disabled with the exact remediable reason. |
| `Unknown` | Detection has not completed or evidence is failed, stale, partial, or contradictory; consequential action is disabled until resolved. |
| `Unsupported` | Yoctui intentionally has no safe maintained implementation for the evidenced behavior/environment; action is disabled with policy and requirement details. |

`Unavailable` is environmental and can change after reconfiguration or
reprobe. `Unsupported` is a Yoctui support decision. Neither may be rendered as
an unexplained generic “Unsupported.” Useful unavailable actions remain visible
for discoverability; features irrelevant to the current workspace may remain
outside that workspace's normal action list.

### Same binary, different connected environments

One installed Yoctui can select different safe outcomes without renderer-local
release checks:

| Connected evidence | Runtime result |
|---|---|
| Devtool help does not expose `upgrade` | `devtool.upgrade` is Unavailable; the visible action is disabled with “Current Devtool does not expose the upgrade subcommand,” and no process is spawned. |
| Initialized `bitbake-getvar` help positively exposes `--value` | `bitbake.getvar` selects `bitbake_getvar.argv` and emits the verified native form. |
| `bitbake-getvar` is absent but `bitbake -e` is positively verified | Variable lookup is AvailableWithLimitations through the maintained environment-dump fallback, with that limitation visible before use. |
| A future release has an unknown name/version | Only directly or negotiated positively evidenced behavior becomes Available; historical options and fallbacks remain Unknown. |

These rows specify product behavior, not a claim that every condition occurred
in the two current live records. Exact observed Scarthgap and Wrynose outcomes
are recorded below and in the release matrix.

### Behavior catalog and implementation alternatives

Capability IDs represent behavior, not versions. The authoritative typed
catalog covers at least these families and is extended without renderer-local
lists:

- BitBake task forcing, environment dump, graph generation, variable lookup,
  signatures/diffsigs/dumpsig, server socket, and native events
- Devtool modify, finish, deploy-target, and upgrade
- Recipetool create and appendfile
- bitbake-layers show-layers and create-layer
- package-data package lookup and path lookup
- Wic create, runqemu, standard/extensible SDK population, menuconfig, and devshell
- CVE checks, SPDX creation, yocto-check-layer, resulttool, and oe-selftest
- buildhistory, locked signatures, and hashserv/prserv diagnostics

Each catalog entry specifies required tools, commands, options, metadata/API
support, direct probes, preferred implementation, optional fallback,
known-but-non-authoritative release boundaries, and the exact default UI reason.
For example, variable lookup may select a native server/API implementation,
then a cataloged older command form, or become unavailable. The command builder
receives the selected implementation, reconstructs validated typed argv, and
rejects absent capability before spawn. It never independently compares a
Yocto or BitBake version.

### BitBake command invocation audit

`BitBakeCommandPlanner` is the only constructor for release-sensitive BitBake
and BitBake-helper arguments. It requires the daemon snapshot's exact build
directory and generation, an enabled behavior record, and the catalog-selected
implementation ID before returning argv. Unknown, unavailable, unsupported,
stale, missing, or mismatched state returns the snapshot reason before a child
can be created.

| Invocation | Capability evidence | Selected command implementation |
|---|---|---|
| Normal image/recipe/task build | `bitbake.build`; task/force additionally requires `bitbake.force_task` | `bitbake.build.argv`; optional verified `-c`/`-f` from `bitbake.force_task.argv` |
| Dependency graph fallback | `bitbake.graph_generation` | `bitbake.graph.argv` emits `-g TARGET` |
| Variable lookup | `bitbake.getvar`; direct executable/help/`--value`/`--recipe` probes for the initialized environment | `bitbake_getvar.argv` runs the separately detected `bitbake-getvar --value [--recipe RECIPE] NAME`, or maintained `bitbake.environment_lookup` runs detected `bitbake -e [RECIPE]` for caller-side lookup; no implementation emits the unsupported `bitbake --getvar` form |
| Environment dump | `bitbake.environment_dump` | `bitbake.environment_dump.argv` emits `-e` |
| Server status/start/stop | distinct `bitbake.server_status`, `bitbake.server_start`, and `bitbake.server_stop` option probes | distinct status/start/stop implementations emit only `--status-only`, `--server-only`, or `--kill-server`; socket/API support alone cannot authorize these CLI options |
| Signature dump | `bitbake.dumpsig` | `bitbake_dumpsig.argv` |
| Signature comparison | `bitbake.diffsigs`, including a direct `-c` option probe | `bitbake_diffsigs.argv` emits the maintained `-c never LEFT RIGHT` form |

The direct `ProcessBackend`, CLI server-control runner, and signature adapter
consume these authorized plans and fail closed when no daemon snapshot is
installed. Base command prefixes are retained only as previously validated
executable/wrapper identity; release-sensitive options and operands come from
the planner. Tinfoil/socket operations do not construct BitBake CLI argv and
are audited separately by `COMPAT-BITBAKE-API-001`. Yocto utilities such as
Devtool, Recipetool, bitbake-layers, pkgdata, Wic, and test tools are separate
catalog families covered by their dedicated compatibility tasks.

The variable-query distinction was verified directly against the official
Wrynose 6.0.2 component composition at OE-Core commit
`5d1aa5c806c061a2994f4decb59016610f093213` and BitBake commit
`acfe02fa38b5da9e6a36c6cedcf91d4fcbefbfbd` (BitBake 2.18.0):
`bitbake --help` exposes no `--getvar`, while initialized-environment
`bitbake-getvar --help` exposes `--value` and `--recipe`, and
`bitbake-getvar MACHINE` returns the configured value. This focused evidence
corrects the command authority; it does not by itself satisfy the separate
latest-release live compatibility gate.

### BitBake backend/API audit

`BitBakeApiAuthority` is the equivalent boundary for Tinfoil, process-server,
socket, metadata, and native-event behavior. It validates the normalized daemon
snapshot, expected generation, exact build directory, and one consistent
selected Tinfoil family. The bridge handshake carries only enabled API
capability IDs and their selected implementations. The bridge directly checks
the initialized environment's callable operations and returns a bounded subset
with the same generation; stale, duplicate, unoffered, or absent behavior is
rejected before a backend command.

| Backend operation | Required behavior capability |
|---|---|
| Workspace, recipes, layers | `bitbake.workspace_inspection`, `bitbake.recipe_inventory`, `bitbake.layer_inventory` |
| Recipe dependencies, sources, metadata | distinct `bitbake.recipe_dependencies`, `bitbake.recipe_sources`, `bitbake.recipe_metadata` |
| Layer relationships | `bitbake.layer_relationships` |
| Effective variables and history | `bitbake.getvar`, `bitbake.variable_history` |
| Native dependency graph | `bitbake.dependency_graph` |
| Build/runqueue/task event flow | `bitbake.build` plus `bitbake.native_events` |
| Cancellation | `bitbake.cancellation` |
| Socket termination/restart transport | `bitbake.server_socket` |

The Python bridge does not classify BitBake by major version. A synthetic
future version may negotiate directly observed behavior, while an older
environment receives only the operations selected by its central snapshot and
confirmed by its connection. Command fallbacks such as `bitbake -e` and `-g`
are not silently executed through the API path. Bridge startup without a
daemon snapshot remains protocol-compatible for diagnostics, but Rust backend
operations fail closed until authority is installed.

### Snapshot ownership and runtime changes

The persistent daemon owns identification, probing, fallback evaluation,
caching, and the monotonically generated `CapabilitySnapshot`. Clients receive
bounded typed identity, capability state, reason code/text, evidence, and
generation over the negotiated protocol. Multiple clients attached to one
daemon observe the same snapshot. Reconnect restores the current snapshot;
clients neither probe nor infer support themselves.

Changing workspace, build directory, BitBake executable/version, initialized
environment, layer configuration/revisions/series, or daemon workspace
invalidates the cache and begins a new generation. Capability state never leaks
between projects. While a new snapshot is incomplete, affected features are
`Unknown`. A client ignores stale generations and revalidates current selection,
open dialogs, and pending actions. An invalidated dialog becomes
non-confirmable or closes with the new reason before any effect is emitted.

Single-process local mode follows the same typed model and probe/catalog code;
it is explicitly degraded because no persistent daemon can preserve shared
snapshot or job state. It must not introduce a second compatibility policy.

### Compatibility evidence requirements

Deterministic evidence must cover identity normalization, every capability
state, catalog completeness, bounded probe success/failure/timeout/truncation,
fallback precedence, old/new argv variants, rejection before spawn, dynamic UI
snapshot replacement, stale generation rejection, and synthetic future
versions. Fixtures must represent the oldest claimed generation, an
intermediate generation, current stable, latest supported, and an unknown
future generation once the support window is established.

A release support claim additionally requires a current, non-fixture live
record with exact official source, repository commit, Yocto series/release,
BitBake version, Yoctui commit, host/build identity, distro, machine,
backend/protocol, commands, observed capabilities, workflow outcomes,
limitations, date, and expiry. Latest-supported evidence must cover environment
detection, probes, workspace, a core build and task/log events, Recipes, Layers,
Configuration, Devtool, representative utilities, and modern BitBake commands.
Older-supported evidence must prove core workflows remain usable, newer absent
features disable with exact reasons, and unsupported argv is never spawned.

Fixture, parser, fake-process, mocked Tinfoil, static table, executable-presence,
or successful compilation evidence cannot satisfy a live claim. Evidence that
predates a relevant capability-contract change or exceeds its expiry is stale
and fails the completion gate. The dedicated Environment/Compatibility Doctor
report exposes the same identity and snapshot used by the UI so a live record
can be audited against runtime behavior.

Run `yoctui --build-dir "$BUILDDIR" doctor` for the human report or append
`--json` for schema `yoctui.doctor.compatibility.v1`. The compatibility section
comes only from the daemon's validated attached snapshot and includes detected
build/release/BitBake/backend/protocol identity, operating mode, all five state
counts, negative executable evidence, and exact limited, unavailable,
unsupported, and unknown feature records. The JSON form also retains the
bounded typed capability records and evidence. A disconnected daemon, absent
snapshot, invalid protocol value, or malformed snapshot is reported as
Unavailable or Invalid; Doctor does not run an independent compatibility probe
or turn fixture/version identity into a release-support claim. Runtime feature
authority remains the snapshot's direct evidence; the separate release-support
classification follows the current non-fixture matrix and its expiry policy.

The reusable deterministic fixture catalog resolves the complete capability
inventory for five explicit policy roles. It covers legacy and modern fallback
boundaries, direct-probe overrides at the latest known boundary, and a
synthetic future generation where only positive direct observations enable a
feature. Every fixture has an exact build/source/tool/backend/protocol identity
with independently Unknown release fields where no authoritative release was
selected. Its `fixture_only` and `deterministic_fixture_only` labels are part of
the test contract, so current-stable/latest candidate slots cannot be mistaken
for live support evidence.

### Current centralized version fallback map

The initial map contains one deliberately narrow unprobeable selector:
`bitbake.tinfoil_adapter`. It may select an adapter family for the core Tinfoil
workspace/recipe/layer/build/cancel/task/server/event capabilities only after
direct backend probes are inconclusive.

| Component range | Fallback implementation | Result |
|---|---|---|
| BitBake `>=1.46,<2.0` | `tinfoil.adapter.legacy` | Available with an explicit version-inference limitation |
| BitBake `>=2.0,<2.19` | `tinfoil.adapter.modern` | Available with an explicit version-inference limitation |
| Missing, malformed, `<1.46`, or `>=2.19` | none | Unknown; positive direct evidence is required |

The official [BitBake release-manual index](https://docs.yoctoproject.org/bitbake/releases.html)
correlates Yocto series with BitBake versions, including Dunfell `1.46`,
Honister `1.52`, Kirkstone `2.0`, Scarthgap `2.8`, and Wrynose `2.18`.
The official [Yocto 4.0 release notes](https://docs.yoctoproject.org/4.0.4/migration-guides/release-notes-4.0.html)
record the Kirkstone BitBake `2.0` branch/revision boundary. These sources
correlate versions; they do not themselves prove every Tinfoil operation. The
fallback therefore stays limited and cannot create a release-support claim.

Any direct positive or negative capability evidence overrides this table.
Conflicting direct evidence, an undeclared catalog selector, and future or
unrecognized versions resolve to Unknown. Renderers and command builders never
parse or compare these versions. The existing Python bridge's broad major-only
adapter selection is migration input for `COMPAT-BITBAKE-API-001`; it is not a
second authoritative map.

## Evidence levels

| Level | Meaning |
|---|---|
| Live observed | The production adapter was run against the exact recorded initialized Yocto build and the stated result was observed. |
| Fixture observed | Reducer, protocol, UI, fake-process, fake-filesystem, or mocked-Python tests exercised behavior without a live Yocto operation. |
| Static/host observed | Compilation, linting, terminal, safety, or analysis evidence was observed without live BitBake control. |
| Not validated | Implementation and fixture coverage exist, but no supported live combination has been recorded. |
| Blocked | A named external prerequisite prevented the evidence command from completing. |

“Fixture observed” is never evidence that a Yocto release, external tool,
device, service, network integration, or artifact layout is compatible.

## Protocol and backend matrix

| Component | Declared range | Validation level | Evidence and limitation |
|---|---|---|---|
| Bridge wire protocol | NDJSON protocol version 1 | Fixture observed and live observed | Framing, sequence/correlation, 1 MiB line bounds, malformed input, unknown messages, and unsupported versions pass deterministic Rust/Python tests. The live snapshot below negotiated version 1. |
| Python Tinfoil bridge | Daemon-selected capability implementations plus direct operation negotiation | Live observed for exact BitBake 2.8.1 and 2.18.0; earlier focused 2.19.0 snapshot | The bridge has no renderer/local major-version switch. It accepts only capability/implementation pairs from the daemon snapshot, negotiates callable operations, and fails closed on stale, unoffered, or absent behavior. |
| Environment-only bridge | Used when Python cannot import a versioned `bb` module | Fixture observed | Supports safe protocol and environment inspection. It is not a build-control adapter and cannot establish live compatibility. |
| Direct process backend | Inherited `bitbake` executable | Fixture/static observed | Shell-free process execution, output bounds, exit, loss, timeout, and process-group cancellation pass fake-process and headless hardening tests. No live Yocto release is currently recorded for this backend. |
| CLI/headless bridge diagnostic | Repository directory without Yocto | Static/host observed | `./scripts/test-cli.sh` and `./scripts/headless-workload.sh target/debug/yoctui bridge` validate startup, bounded handshake/shutdown, isolated session state, and explicit absent daemon compatibility authority. They do not run a workspace API or live BitBake without a daemon snapshot. |

## Current latest-stable live evidence

On **2026-08-19**, the authoritative Yocto release calendar identified 6.0.2
as the newest published stable Wrynose release; 6.0.3 was still scheduled for
the following week. The official 6.0.2 release notes identify the exact
OE-Core, BitBake, and meta-yocto commits recorded in
[`compatibility-evidence/latest.toml`](compatibility-evidence/latest.toml).

The production daemon detected Poky/DISTRO 6.0.2, Wrynose OE-Core series,
BitBake 2.18.0, qemux86-64, the exact configured build/layers/tools, and
protocol 1. Doctor reported the same generation-one snapshot. Capability-aware
headless clients installed that daemon snapshot before backend construction;
they did not probe or infer release support independently.

Live scope included 1,922 recipes, three configured layers, `MACHINE` through
the selected `bitbake-getvar --value` implementation, Devtool/Recipetool/
bitbake-layers/pkgdata command surfaces, a successful
`base-files:do_listtasks` daemon build with typed native events, and a separate
`core-image-minimal` cancellation that reached one shared terminal in 0.257
seconds without force kill. A default `base-files` build entered upstream fetch
work and was cancelled rather than treating network completion as compatibility
evidence. `pkgdata.generated` remained honestly Unavailable, and incomplete
Devtool probes remained Unknown even when manual help was retained as live
diagnostic evidence.

The exact revision is the current **Claimed supported** upper anchor in the
release matrix. Individual runtime actions still require connected-environment
capability evidence.

## Current older-LTS live evidence

On **2026-08-19**, authoritative Yocto release policy identified Scarthgap 5.0
as a maintained LTS through April 2028, and the release calendar showed 5.0.19
as published while 5.0.20 remained scheduled. The official 5.0.19 release
notes identify Poky commit `bb98354685781296e3b3737e7762412100f359c2`,
OE-Core `2814f0962f56c8d1afa4de76d2895ba9b5cb767d`, BitBake
`0880963fea4d91a034e4a6e007d23f98658ab986`, and meta-yocto
`2f749ae477c3b94dce71038f025180d7f612dab0`.

The production daemon detected Scarthgap/Poky 5.0.19, BitBake 2.8.1,
qemux86-64, three configured layers, initialized tools, and protocol 1 from a
fresh official Poky checkout. Doctor exposed the same snapshot in degraded
mode: 36 Available, 14 AvailableWithLimitations, four Unavailable, 22 Unknown,
and no Unsupported capabilities. Exact reasons identify absent build-compare
and sstate-cleanup tooling, missing generated pkgdata, and incomplete utility
probes; Unknown actions have no selected implementation.

Live scope included 1,829 Recipes, three Layers, `MACHINE=qemux86-64` through
the positively probed `bitbake-getvar --value` implementation, the initialized
Devtool/Recipetool/bitbake-layers/pkgdata surfaces, and a successful
`base-files:do_listtasks` daemon run with 77 typed workspace/parse/task/log/
completion observations. A separate `core-image-minimal` cancellation exposed
and fixed older-event ordering, then reached the shared Failed terminal in
0.405 seconds. No `bitbake --getvar` form or another unsupported argv was
emitted. The exact machine-readable scope is in
[`compatibility-evidence/older.toml`](compatibility-evidence/older.toml).

This exact maintained revision is the current **Claimed supported** lower live
anchor. The claim covers the recorded required workflow set and does not infer
support for an untested Scarthgap point revision.

## Current one-stop workbench live evidence

On **2026-08-27**, the production release binary was exercised through real
PTYs while a complete `core-image-minimal` build ran on a supported Yocto host:

| Field | Recorded value |
|---|---|
| Host | Ubuntu 24.04.4 LTS, glibc 2.39, Linux x86_64 |
| Poky | 5.2.4, branch `yocto-5.2.4`, commit `d0b46a6624ec9c61c47270745dd0b2d5abbe6ac1` |
| BitBake | 2.12.1 |
| Machine / target | `qemux86-64` / `core-image-minimal` |
| Yoctui source | `879f9ee3c922dc30b7f11048f7dc398b7072d30d`, a verified ancestor of the current checkout |
| Release binary SHA-256 | `2578e1d8060b2fd56d5e9b303bbd742491ddc13f2840cd5bcbdf2a8bd6ace432` |
| UI scenarios | Startup, environment, Layers, Recipes, Tasks, live logs, completion, safe failure, menus/availability, manifest/pkgdata/rootfs, interactive task availability, context terminal, and daemon reconnect passed |
| Image evidence | 38 manifest packages and 14,995 pkgdata files; exact manifest SHA-256 retained |
| Filesystem evidence | `Unavailable (cleaned)`: `rm_work` removed transient `IMAGE_ROOTFS`; no logical-filesystem total is claimed |

This run establishes the interactive one-stop-workbench workflow for this exact
host/release/binary boundary. It complements rather than replaces the newer
6.0.2 and older-LTS 5.0.19 release anchors: those records establish release
compatibility roles, while this run establishes complete UI behavior and a
successful image build. `./scripts/verify-live-workbench-ux-evidence.sh`
requires the evidence to remain within 90 days and its source commit to remain
an ancestor of `HEAD`.

## Observed live Yocto combination

The following combination was observed on **2026-07-24** using the production
Python bridge and Tinfoil:

| Field | Observed value |
|---|---|
| BitBake | `2.19.0` |
| Distribution | `poky` |
| Yocto release | `6.0.99+snapshot-a4eb7bc2a750f76d9772eb88b7afb2b801bd1250` |
| Machine | `qemux86-64` |
| Normal smoke operation | `base-files:do_listtasks` |
| Cancellation target | `core-image-minimal` |
| Backend | Python bridge, modern Tinfoil adapter, protocol 1 |
| Host identity | Not recorded; no host-distribution support claim is derived from this run |

Reproduce the core live smoke only in an initialized, disposable or otherwise
approved build environment:

```sh
export YOCTUI_LIVE_BITBAKE=1
export YOCTUI_LIVE_BUILD_DIR="/absolute/path/to/initialized/build"
./scripts/verify-live-bitbake.sh
```

The wrapper accepts a bitbake-setup `build/init-build-env`, an already sourced
matching `BUILDDIR`, or `YOCTUI_OE_INIT_BUILD_ENV` naming a complete Poky
`oe-init-build-env`. It checks `conf/local.conf`, `conf/bblayers.conf`, the
`bitbake` executable, and Python `bb.tinfoil` before starting. Defaults may be
overridden with `YOCTUI_LIVE_TARGET`, `YOCTUI_LIVE_TASK`,
`YOCTUI_LIVE_CANCEL_TARGET`, and `YOCTUI_LIVE_TIMEOUT`.

### Live capability evidence from that snapshot

| Capability family | Level | Observed evidence |
|---|---|---|
| Handshake and workspace | Live observed | Bridge version, release, exact build directory, `MACHINE`, configured layer inventory, and clean shutdown were returned through correlated protocol events. |
| Recipe inventory/detail | Live observed | `base-files` resolved to version `3.0.14`, zero applied appends, 39 authoritative tasks, one metadata source, and four package outputs. |
| Normal build event flow | Live observed | Parse progress, queued/started task events, a positive authoritative task total, logs, and successful `base-files:do_listtasks` completion were observed. |
| Cancellation event flow | Live observed | `core-image-minimal` started, received cancellation, and completed with a non-success cancellation result. |
| Dependency graph | Live observed | Tinfoil `generateDepTreeEvent` returned 962 typed nodes and 1,779 build/runtime/task edges with task edges present. |
| Configuration query | Live observed, focused | `MACHINE` and `OVERRIDES` queries retained effective/unexpanded values, provenance, operations, and active overrides. A copy of live `local.conf` passed atomic-writer validation; the live file itself was not changed. |
| Signatures | Live observed, separate opt-in test | Real dumps produced one `autoconf-native:do_fetch` record and one `do_recipe_qa` record; comparison returned 113 typed differences, no dump limitation, and one explicit recursive-diffsigs limitation. |

The core harness validates the first five rows on every opt-in run. The
configuration and signature rows are separately recorded focused evidence;
they are not silently implied by the core harness.

Run the copy-only configuration writer validation with:

```sh
YOCTUI_LIVE_BUILD_DIR="$YOCTUI_LIVE_BUILD_DIR" \
cargo test -p yoctui config_edit_write_live_snapshot -- --ignored --nocapture
```

Run the opt-in signature adapter smoke after sourcing the same environment:

```sh
export YOCTUI_LIVE_DUMPSIG="$(command -v bitbake-dumpsig)"
export YOCTUI_LIVE_DIFFSIGS="$(command -v bitbake-diffsigs)"
cargo test -p yoctui-bitbake --test live_signature -- --ignored --nocapture
```

These observations apply only to the exact snapshot and operations listed.
They are not a claim that every BitBake 2.x, Poky snapshot, machine, distro,
recipe, task, or external tool is supported.

## Workflow compatibility matrix

| Workflow | Validation level | Current evidence or blocker |
|---|---|---|
| Persistent shell, responsive UI, menus, dialogs, Tasks, Logs, Errors, Terminal Sessions | Live observed for the M21 boundary; fixture/static observed across the full matrix | The 2026-08-27 PTY run recorded active tasks, logs, successful completion, safe failure, contextual menus, a daemon-owned build shell, and reconnect. Breakpoint, accessibility, focus, and invalid-context breadth remains deterministic fixture/static evidence. |
| Layers and Recipes | Live observed for inventory/detail; fixture observed for editing | Live configured layers and `base-files` metadata passed. Lazy trees, previews, editors, path rejection, and external-editor lifecycle are fixture/static evidence. |
| Configuration | Live observed for reads; copy-only validation for writes | Live value/provenance data passed. Atomic edit logic was applied only to a temporary copy of live `local.conf`; no live metadata write is claimed. |
| Devtool | Not validated | Exact command construction, status, Git state, editor, cancellation, and terminal outcomes use fake processes/filesystems. No live `devtool modify/update/finish/deploy/reset` combination is recorded. |
| Dependencies | Live observed for `base-files` | The live graph counts above passed through the modern Tinfoil API. Legacy fallback and process `bitbake -g` behavior are fixture-only. |
| Signatures | Live observed for the two recorded tasks | Other recipes, tasks, histories, releases, and recursive detail remain unvalidated. |
| Package data and Rootfs composition | Live observed for the M21 image; fixture observed for traversal breadth | The 2026-08-27 build recorded 38 exact manifest packages and 14,995 pkgdata files. `rm_work` removed `IMAGE_ROOTFS`, which passed as an explicit cleaned/unavailable state; bounded logical traversal remains fixture evidence rather than a claimed live filesystem total. |
| Deployed Images | Live observed for the selected image manifest; fixture observed for general scanning/actions | `core-image-minimal` manifest association passed for qemux86-64. Other artifact formats, opening, cancellation, and failure paths retain fake-filesystem coverage; no blanket deploy-operation claim is made. |
| SDK | Not validated | Populate/test request routing, artifact scans, publication, native tools, and lifecycle use typed fixtures and fake processes only. |
| QEMU | Not validated | `runqemu` discovery, exact arguments, output, cancellation, and loss use fake artifacts/processes. No live boot is recorded. |
| Wic create and device write | Not validated | Command, kickstart, generated-output, fake-device safety, revalidation, and cancellation tests pass. No live Wic tool or removable-media write is recorded. |
| Testing/resulttool | Not validated | Selftest/task routing, imports, comparison, JUnit, cancellation, timeout, and loss use fixtures/fake processes. No live test suite/resulttool combination is recorded. |
| Security | Not validated | CVE/SBOM capability, exact task routing, bounded reports, package mapping, and cancellation use fixtures/fake processes. No live Yocto security configuration or report layout is recorded. |
| QA | Not validated | Recipe/kernel and layer checks, reports, source opens, lifecycle, and cancellation use fixtures/fake processes. No live QA tool/release combination is recorded. |
| Maintenance | Not validated | Sstate, PR/hash diagnostics, release evidence, and detection-only integrations use fixture/process evidence. No live cache deletion, PR database change, archive/network operation, service control, or optional integration is claimed. |

## Host, runtime, and hardening matrix

| Environment | Date | Level | Evidence and limitation |
|---|---|---|---|
| Ubuntu 24.04.4 LTS, glibc 2.39, Linux x86_64 | 2026-08-27 | Live observed for exact M21 combination | A complete Poky 5.2.4 `core-image-minimal` build and real-PTY workbench scenarios passed. This exact host is the recorded live-build boundary; it does not imply support for every distribution or libc. |
| GitHub Actions `ubuntu-latest`, stable Rust, Python 3.12 | Current CI definition | Static/host observed | Format, Clippy, workspace tests, terminal, stress, CLI, Python unit/static/coverage gates run without a real Yocto checkout. `ubuntu-latest` is not a pinned production host claim. |
| Linux `7.0.0-28-generic` x86_64, Rust/Cargo 1.97.0, Python 3.14.4 | 2026-08-01 | Static/host observed | Workspace tests, Clippy, Python tests, terminal, stress, ASan/LSan, coverage, security checks, Valgrind, and deterministic profile gates have passed on this development host. This is not the recorded live-snapshot host identity. |
| Linux pseudo-terminal | Current baseline | Static/host observed | Alternate-screen and cursor hide/show restoration pass `./scripts/test-terminal.sh`. macOS, BSD, native Windows, and WSL terminal matrices are not recorded. |
| Fuzzing | Current baseline | Static/host observed | Finite cargo-fuzz smoke covers protocol frames and retained logs. Finite fuzzing is not exhaustive. |
| Valgrind | Current development host | Static/host observed | No definite, indirect, or possible lost bytes were reported; two Tokio signal descriptors are explicitly recognized. Still-reachable allocations are reported separately. |
| Flamegraph | 2026-08-15 | Static/host observed | With explicitly authorized temporary `kernel.perf_event_paranoid=0`, `cargo-flamegraph 0.6.13` and matching `perf 7.0.12` captured real userspace samples and regenerated the deterministic headless SVG. Restricted kernel symbols remain a reported host limitation. |

Reproduce the Flamegraph blocker with:

```sh
perf record --no-buildid-mmap -e dummy:u -o /tmp/yoctui-perf.data -- true
./scripts/flamegraph.sh
```

Under local security policy, grant `CAP_PERFMON` to the matching `perf` binary
or temporarily run `sudo sysctl -w kernel.perf_event_paranoid=0`, then verify:

```sh
./scripts/flamegraph.sh
test -s artifacts/flamegraph/yoctui.svg
./scripts/verify-completion.sh
sudo sysctl -w kernel.perf_event_paranoid=4
```

Until that succeeds, the hardening analysis gate remains blocked. Tool
installation alone is not a pass.

## Adding a supported live combination

1. Use an initialized disposable or approved Yocto build. Record the host OS,
   architecture, setup source/revision, BitBake version, Yocto release,
   distro, machine, backend, and Yoctui commit before testing.
2. Run the ordinary baseline from [Testing](testing.md). A baseline failure is
   not waived by a successful live operation.
3. Run the opt-in core command exactly:

   ```sh
   YOCTUI_LIVE_BITBAKE=1 \
   YOCTUI_LIVE_BUILD_DIR=/absolute/path/to/build \
   ./scripts/verify-live-bitbake.sh
   ```

4. Preserve the final JSON summary and record the chosen normal/cancellation
   targets. Confirm that the build directory and configuration files were the
   intended ones and that cancellation reached a terminal result.
5. Run only the workflow-specific opt-in tests whose prerequisites and side
   effects were reviewed. Use disposable caches, repositories, services, and
   removable devices for destructive or network validation.
6. Add one matrix row with the date, exact versions/identities, exact command,
   observed capability, limitations, and evidence level. Do not broaden a row
   to a major-version or tool-family claim without separate representative
   evidence and an explicit policy change.

## Related guidance

- [Installation and guarded quickstarts](../README.md)
- [Daily operator workflows and troubleshooting](operator-guide.md)
- [Testing and opt-in live validation](testing.md)
- [Profiling and analysis prerequisites](profiling.md)
- [Protocol contract](protocol.md)
- [Architecture and authority boundaries](architecture.md)
