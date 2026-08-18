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
`compatibility-matrix.md`; it is not compiled into UI conditionals. At the
start of M18, **no minimum supported release is claimed** because the required
older-release live gate has not run. The **latest tested exact environment** is
the Poky development snapshot and BitBake `2.19.0` observation recorded below,
classified only as partially tested; it is not yet the latest *supported*
release. `COMPAT-LIVE-OLDER-001` establishes the minimum and
`COMPAT-LIVE-LATEST-001` establishes the latest supported stable only after
both satisfy the machine-readable evidence policy.

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
| Variable lookup | `bitbake.getvar` | `bitbake.getvar.argv` emits `--getvar NAME`, or maintained `bitbake.environment_lookup` emits old-compatible `-e` for caller-side lookup |
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
| Python Tinfoil bridge | BitBake major 1 selects the legacy adapter family; major 2 or later selects the modern family | Live observed only for BitBake 2.19.0 | Adapter-family selection localizes API differences. Recognizing a major version is not a compatibility claim for every release in that family. Malformed and pre-1 values fail with `unsupported_bitbake`. |
| Environment-only bridge | Used when Python cannot import a versioned `bb` module | Fixture observed | Supports safe protocol and environment inspection. It is not a build-control adapter and cannot establish live compatibility. |
| Direct process backend | Inherited `bitbake` executable | Fixture/static observed | Shell-free process execution, output bounds, exit, loss, timeout, and process-group cancellation pass fake-process and headless hardening tests. No live Yocto release is currently recorded for this backend. |
| CLI/headless bridge smoke | Repository directory without Yocto | Static/host observed | `./scripts/test-cli.sh` and `./scripts/headless-workload.sh target/debug/yoctui bridge` validate startup, protocol inspection, session isolation, and shutdown only. They do not run live BitBake. |

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
| Persistent shell, responsive UI, dialogs, Tasks, Logs, Errors | Fixture/static observed | Reducer/input and Ratatui TestBackend coverage pass at supported breakpoints; Linux pseudo-terminal restoration passes. The live bridge emitted task/log/build events, but the complete interactive TUI was not recorded as an end-to-end live matrix run. |
| Layers and Recipes | Live observed for inventory/detail; fixture observed for editing | Live configured layers and `base-files` metadata passed. Lazy trees, previews, editors, path rejection, and external-editor lifecycle are fixture/static evidence. |
| Configuration | Live observed for reads; copy-only validation for writes | Live value/provenance data passed. Atomic edit logic was applied only to a temporary copy of live `local.conf`; no live metadata write is claimed. |
| Devtool | Not validated | Exact command construction, status, Git state, editor, cancellation, and terminal outcomes use fake processes/filesystems. No live `devtool modify/update/finish/deploy/reset` combination is recorded. |
| Dependencies | Live observed for `base-files` | The live graph counts above passed through the modern Tinfoil API. Legacy fallback and process `bitbake -g` behavior are fixture-only. |
| Signatures | Live observed for the two recorded tasks | Other recipes, tasks, histories, releases, and recursive detail remain unvalidated. |
| Package data | Blocked live attempt | The selected live build lacked `build/tmp/pkgdata`; no live package inventory is claimed. Build a target through `do_package`, then run `YOCTUI_LIVE_BUILD_DIR=/path cargo test -p yoctui-bitbake --test live_pkgdata -- --ignored --nocapture`. |
| Deployed Images | Not validated | Bounded deploy scanning, typed artifact association, opening, cancellation, and failure paths use fake filesystems. No live deployed-artifact layout is recorded. |
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
