# Compatibility and Validation Evidence

Yoctui requires stable Rust for compilation and an initialized Yocto
environment for production BitBake control. This document records observed
evidence, not compatibility inferred from version numbers. The authoritative
metadata and build result always come from the selected BitBake environment.

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
