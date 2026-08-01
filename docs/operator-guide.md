# Yoctui Operator Guide

This guide covers daily use after a Yocto environment has been initialized and
Yoctui has opened its build directory. Follow the guarded setup commands in the
[README](../README.md) first. BitBake, its configured metadata, and adapter
events remain authoritative; a visible row, filename, or log message is never
treated as proof that an operation or artifact exists.

## Start a workspace safely

Source the setup script for exactly one Yocto layout, verify `BUILDDIR`, and
launch without a positional target:

```sh
test -n "${BUILDDIR:-}" || { echo "BUILDDIR is not set" >&2; exit 1; }
cd "$HOME/projects/yoctui"
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR"
```

Opening a workspace does not start a build. The bridge backend is the normal
choice for typed workspace metadata and live events. Use `--backend process`
only as a compatibility fallback; it can expose fewer capabilities. A mocked
or environment-only bridge is useful for tests but is not live BitBake
control. See [Compatibility](compatibility.md) for observed live evidence.

## Understand the persistent shell

The header keeps the active build, backend, target, `MACHINE`, `DISTRO`, task
counts, warning/error counts, elapsed time, CPU use, and build-filesystem free
space visible when that data is available. The left Navigator selects a
workspace, the center pane contains its rows or tree, and the Inspector shows
the exact selected identity. The footer is the authoritative shortcut list for
the currently focused pane; disabled actions remain visible with a reason.

- `Up`/`Down` or `j`/`k` moves the selection in the focused pane.
- `Enter` activates the selected row or the primary dialog action.
- `Tab` and `Shift+Tab` move between Navigator, Workspace, and Inspector.
- `Ctrl+P` opens the searchable command palette. Unavailable commands explain
  their prerequisites and remain inert.
- `Esc` leaves a search, closes a dialog, moves outward, or cancels the current
  transient mode. Dialogs trap focus until closed.
- `q` requests application exit. Exiting Yoctui and cancelling a build are
  separate operations.
- `!` suspends the TUI and opens the inherited Yocto shell. Run `exit` to
  restore Yoctui.

At 130 columns or wider all three panes are visible. From 100 through 129
columns the Inspector appears as a focusable overlay. From 80 through 99
columns one pane is visible at a time. Below 80x24, resize the terminal or
press `q`; Yoctui deliberately renders no partial workspace.

## Daily image-build loop

1. Confirm the active release, build directory, `MACHINE`, and `DISTRO` in the
   header and Inspector. If they are unknown, diagnose the environment before
   building.
2. Press `B` for image build options, then `e` to set the target. For the first
   smoke image, enter `core-image-minimal`.
3. Review the exact target, task, machine, backend, and any unusual options in
   the confirmation. Only the final `Enter` starts BitBake; `Esc` makes no
   request.
4. Continue working while the background build runs. Tasks shows authoritative
   completed/total progress and honest indeterminate activity. Dashboard and
   the header retain CPU and build-filesystem telemetry.
5. Use Logs for retained output and Errors for structured warnings or failures.
   Ordinary logs may be coalesced or evicted under pressure, but visible drop
   counters and protected failure records preserve that limitation.
6. Inspect the completion dialog. Success, warning-only completion, failure,
   cancellation, timeout, and backend loss are different terminal outcomes.
   A failed completion can open the exact retained diagnostic in Errors.

`F5` or the contextual build command may open the normal build dialog. A
recipe selected in Recipes uses `b` to build that recipe, not the current
image. Recipe task, forced, clean, and unusual requests always show their exact
intent before execution.

## Core workspaces

| Workspace | Daily use | Important controls and evidence |
|---|---|---|
| Dashboard | Check build state, recent builds, telemetry, diagnostics, and common actions. | During a build, move to Tasks, Logs, or Errors without interrupting it. |
| Tasks | Follow active, waiting, completed, and failed task state. | `f` cycles state filters, `F` selects a text-filter field, `/` edits it, and `d` cycles duration thresholds. Unknown progress is labelled unknown rather than shown as 0%. |
| Logs | Follow bounded live output and inspect exact selected records. | `f` follow, `w` wrap, `/` search, `n`/`N` matches, `s`/`R`/`T`/`B` filters, `o` source log, `C` copy when a clipboard helper is available. |
| Errors | Inspect structured warnings, errors, backend loss, and suggested actions. | `Enter` opens exact retained log context; `o` opens an authoritative source path. Missing source identity stays disabled. |
| Images | Select image recipes and inspect deployed artifacts for the active machine. | `i` target picker, `b` confirmed build, `R` rescan, `c` cancel scan, `o` artifact, and `m`/`l`/`s`/`w` exact associated manifest/license/SPDX/Wic paths. A buildable target and a deployed artifact are separate facts. |
| Packages | Query generated package data and follow runtime relationships. | `R` inventory, `Enter` detail, `/` search, `D` dependency direction, `[`/`]` dependency, `d` follow, `u` back, `o` recipe, `e` provider, `c` cancel. Missing `tmp/pkgdata` means a target must complete `do_package`. |

## Browse and edit layers, recipes, and configuration

### Layers

Layers lists only metadata layers reported as configured for the active build.
Their active/selected styling, priority, compatibility, and Git decorations
are context rather than an independent reconstruction of `bblayers.conf`.

- Select a layer and press `Enter` to open its lazy metadata tree. Directories
  load only when expanded; `Right`/`l` descends, `Left`/`h` collapses or moves
  upward, `.` toggles hidden entries, `/` searches, and `r` refreshes the
  selected subtree.
- Select a layer and press `e` for the large in-TUI two-pane editor. The tree
  stays on the left and the syntax-aware preview/editor stays on the right.
  `Enter` or `e` edits a selected file, `Ctrl+S` saves, and `Esc` returns.
- Press `o` from the configured-layer inventory to use the configured external
  editor. Yoctui suspends and restores the terminal around that process.
- In the layer browser, `g`, `m`, and `d` switch Inspector context among Git,
  metadata, and relationships. Binary and oversized previews remain bounded.

### Recipes and Devtool

Recipes lists provider-resolved names, versions, layers, appends, and any
authoritative build or Devtool status. `/` searches and `Enter` refreshes the
selected recipe's tasks, sources, patches, packages, and status.

- `b` confirms a default recipe build. `f` chooses an authoritative task and
  force intent. Context actions for clean, cleansstate, devshell, menuconfig,
  diffconfig, and diffsigs stay disabled unless the recipe reports that task.
- `e` opens the exact provider, `o` selects an authoritative task log, and `p`
  selects a resolved local patch. Yoctui never guesses these paths.
- `g` opens the typed Dependencies workspace. `Z` opens signature history for
  an authoritative recipe/task; choose sides with `1` and `2`, then `c` to
  compare. Lower-case `z` remains the confirmed BitBake `diffsigs` task.
- `t` refreshes Devtool status. `d` previews `devtool modify` when needed or
  opens the exact reported workspace source. `u`, `F`, `P`, and `D` route to
  update-recipe, finish, deploy-target, and destructive reset respectively.
  Each route validates current status and shows the exact operation first.
- Devtool source editing uses the two-pane editor. `Ctrl+S` saves. `Ctrl+B`
  refuses dirty editor content, then closes the editor and opens a confirmed
  build for that recipe; it never silently builds the image.

Devtool jobs run independently from the managed BitBake build and retain
bounded stdout/stderr after navigation. A missing executable, missing source,
dirty finish workspace, nonzero exit, rejected cancellation, or lost runner is
shown as a distinct state. Cancellation targets only the exact active job.

### Configuration and BBMASK

Configuration shows effective and unexpanded values, scope, provenance,
overrides, and operations supplied by BitBake. `Enter` loads detail; `s`
chooses scope; `c` compares exact scopes; `o` opens a reported defining source;
copy actions remain disabled without a clipboard helper.

Configuration is read-only by default. Only the documented allowlisted global
variables can open `E`; the second dialog shows the exact assignment and
`$BUILDDIR/conf/local.conf` destination before an atomic write. Recipe-scoped,
unloaded, unsupported, multiline, or stale values never write. The BBMASK view
uses the same preview/confirmation principle and reports provenance; it does
not silently change masking.

## Dependency, package, and signature evidence

Dependencies renders only normalized recipe/task nodes and reported build,
runtime, or task edges. `Up`/`Down` selects, `Enter` opens an owning recipe,
`o` opens an exact provider, `L` opens an exact task log, and `r` refreshes the
same root. The Inspector distinguishes a valid empty graph, a partial graph,
failure, unreachable nodes, cycles, and bounded why-built paths.

Package data comes from validated `oe-pkgdata-util` operations; signature
records come from validated dumpsig/diffsigs artifacts. Their typed summaries
are evidence. Fixture tests establish parsing, bounds, and lifecycle behavior
only; check [Compatibility](compatibility.md) before relying on a workflow in a
production release.

## Image, SDK, QEMU, and Wic operations

The SDK workspace binds every action to the exact active image, machine, and
distro. `s` and `E` preview standard and extensible SDK tasks; `t` and `T`
preview their test tasks; `R` rescans the authoritative SDK deploy root. `P`
publishes a selected installer only after an indexed command preview. `n`
previews a shell-free native-tool operation whose environment is confined to
the child. `c` cancels only the SDK-owned operation.

From a compatible selected artifact in Images, `Q` opens the typed QEMU form.
Review image, machine, kernel/rootfs overrides, networking, display, serial,
memory, and bounded extra keywords; `p` opens the exact preview and only its
final `Enter` launches. `x` confirms cancellation of the exact managed QEMU
session. Missing `runqemu` or incompatible artifacts never becomes a guessed
command.

`W` opens cooked-mode Wic creation for an exact image and reported kickstart.
Review output directory, bmap, compression, source summary, and the indexed
command preview. Generated output remains correlated to that request. `D`
opens protected device writing only for an eligible whole removable device.
It requires the exact `WRITE <device-path>` phrase and a separate destructive
confirmation, revalidates image and device immediately before spawn, and never
invokes `sudo`. Fixture device tests do not prove live hardware safety.

## Testing, Security, and QA

These workspaces expose fixed typed operations, not a generic shell textbox.
Unavailable rows explain missing tasks, tools, configuration, roots, or
artifacts.

- Testing separates Launches, Results, and Comparison. Launches cover
  `oe-selftest`, `bitbake-selftest`, image runtime, SDK tests, and configured
  ptests. Every launch has an indexed request preview. Structured result import
  accepts exact paths; comparison uses exact fingerprints; JUnit export refuses
  an existing destination. `x` cancels only the Testing-owned operation.
- Security separates CVEs and SBOM. Exact capability determines recipe/image
  scope and current versus legacy task names. Checks, mapping, generation,
  import, and rescans retain report fingerprints and bounded limitations.
  Yoctui never enables missing security configuration or invents report roots.
- QA separates recipe/kernel checks and layer QA. Launches, report imports,
  findings, source/provider opens, and cancellation remain bound to exact typed
  identities. Layer QA runs independently from managed BitBake checks.

Use each workspace's contextual footer for view switching, search, refresh,
open, and cancel keys. A successful command with no adapter-reported result is
shown as success with no report, not as fabricated evidence.

## Maintenance

Maintenance has Sstate, Services, Release, and Integrations views. `[` and `]`
change view, `r` refreshes capability, `Enter` inspects, `x` requests exact-job
cancellation, and `o` opens validated successful evidence.

- Sstate `c` previews `oe-check-sstate`. Destructive cleanup `d` first obtains
  exact candidates, then requires the displayed `DELETE ... FROM ...` phrase
  and another confirmation. Changed candidates reject execution.
- Services reports PR/hash configuration and observational process evidence;
  it never starts or stops services. PR export/import shows native side effects
  and exact paths before confirmation.
- Release covers locked-signature cache generation, build-history comparison,
  and Git archive evidence. Replacements are explicit. A requested archive
  push is a second network confirmation after local success.
- Integrations detects pull-request, error-report, manifest, and Toaster tools
  only. This view does not send mail, upload, mutate manifests, or manage
  Toaster.

Maintenance fixture coverage does not establish live cache safety, service
health, PR database compatibility, archive correctness, or network
interoperability. Use disposable resources for any explicit live validation.

## Background jobs, cancellation, and terminal outcomes

Builds, Devtool, QEMU, Wic, SDK, tests, QA, Security, and Maintenance operations
retain stable job identities, bounded output, timestamps, progress when known,
warnings/errors, and terminal outcomes. You may navigate or edit unrelated
files while they run. Returning to the owning workspace restores the retained
context.

Cancellation never means success or generic failure. Request cancellation from
the owning workspace, review destructive interruption warnings, and wait for
acknowledgement or an explicit rejection/forced outcome. Never assume that
closing Yoctui cancelled an external operation. If a backend or runner is lost,
inspect retained output and the exact external state before retrying.

## Settings, configuration, and sessions

Settings supports theme, animation speed, reduced motion, color, log wrapping,
and log following. `Up`/`Down` selects and `Left`/`Right` or `Enter` changes a
value. Changes apply immediately and are atomically saved to `session.toml`;
`r` retries a failed save.

Startup precedence is CLI flags, `YOCTUI_*` environment variables,
`$XDG_CONFIG_HOME/yoctui/config.toml`, the recent session, then built-in
defaults. Interactive settings in `session.toml` override configuration
defaults, while hard CLI choices such as `--no-color` stay authoritative.
Deleting `session.toml` resets remembered UI state but never resets BitBake.
For CI, set a temporary `XDG_CONFIG_HOME` so developer targets and build paths
cannot enter an automation run; `scripts/headless-workload.sh` demonstrates
that pattern.

## Troubleshooting

### Release is unknown or recipes/layers are empty

Quit without starting a build. Confirm that the correct environment setup
script was sourced in the same shell and that `BUILDDIR` names its actual
build directory:

```sh
test -n "${BUILDDIR:-}"
test -d "$BUILDDIR/conf"
command -v bitbake
bitbake --version
cargo run --locked -p yoctui -- doctor
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR" inspect
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR" layers
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR" recipes
```

Do not source standalone BitBake's `init-build-env` using a Poky path, or expect
`oe-init-build-env` in the README-only Poky `master` migration repository. Use
the separate setup blocks in the README.

### A capability, tool, or artifact is unavailable

Read the disabled reason in the Inspector. Verify the tool is on `PATH` in the
initialized shell and that the selected release actually reports the required
task or configuration. Build prerequisites such as `do_package`, SDK, image,
or report generation may need to complete before an artifact inventory exists.
Refresh the owning workspace after correcting the external state. Do not work
around canonical-path or symlink rejection by copying a displayed command.

### A build or job failed, timed out, was cancelled, or was lost

Open its owning workspace, then Logs and Errors. Record the exact target/task,
backend, exit status, retained stdout/stderr, dropped-output counters, and
evidence identity. A lost runner or backend disconnect has unknown external
state; inspect the actual process, output, cache, device, or repository before
retrying. Never interpret an empty report as success when the UI says failed or
partial.

### The external editor or shell does not return cleanly

Set `editor` in configuration or ensure `$EDITOR` and `$SHELL` name executable
programs. Exit the child normally. If an unexpected process termination leaves
the calling shell visually damaged, run:

```sh
reset
stty sane
```

Then rerun `cargo run --locked -p yoctui -- doctor`. Reproduce terminal issues
with `./scripts/test-terminal.sh`; see [Testing](testing.md) before reporting a
terminal-lifecycle defect.

### Diagnostics or analysis prerequisites are missing

`./scripts/verify-completion.sh` is deliberately strict and names missing
coverage, security, sanitizer, Valgrind, or profiling requirements. Follow
[Testing](testing.md) and [Profiling](profiling.md); do not treat a skipped
optional tool as a passing release gate.

## Reference boundaries

- [README installation and quickstart](../README.md)
- [Authoritative UI behavior](ui-spec.md)
- [Architecture and authority boundaries](architecture.md)
- [Protocol](protocol.md)
- [Live and fixture compatibility evidence](compatibility.md)
- [Testing](testing.md)
- [Profiling](profiling.md)
