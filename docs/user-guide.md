# User guide

[Install Yoctui](../README.md#install), then open your initialized Yocto build.
Opening a workspace does not start a build. The footer and F1 show current controls.
Use `yoctui --help` for CLI options and [screenshots](screenshots.md) for the gallery.

## Installation requirements

Linux, an 80×24+ terminal, Python 3, a C compiler/linker, and
[stable Rust/Cargo](https://www.rust-lang.org/tools/install) are required.
After the [official install](../README.md#install), add Cargo's bin directory to PATH:

```bash
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
```

Persist that setting in your shell configuration if needed. For unpublished changes,
see [building from source](development.md#build-from-source). Reopen the client after
updates; restart the daemon only after its builds and sessions finish.

## Start a workspace safely

For a complete Poky checkout, replace the paths and run in Bash:

```bash
export POKY_DIR="$HOME/src/poky"
export BUILDDIR="$POKY_DIR/build-yoctui"
test -f "$POKY_DIR/oe-init-build-env" || {
  echo "Missing oe-init-build-env; check the source layout." >&2
  exit 1
}
source "$POKY_DIR/oe-init-build-env" "$BUILDDIR"
yoctui daemon status
```

If status names a different workspace, finish its jobs/sessions before switching.
Otherwise use `yoctui daemon start`, then
`yoctui --backend bridge --build-dir "$BUILDDIR"`; later visits use `yoctui attach`.
Run as the build user. Split layouts may use layers/openembedded-core; BSPs can
require a vendor setup wrapper. Configure MACHINE and parallelism before building.
An existing daemon is not retargeted by daemon start.

Without an initialized shell, open **Build Environment**, press `e`, and select
Source/Build/Script. `b` browses, `s` saves, and `V` explicitly initializes/verifies.
Use an existing build directory; custom scripts stay editable. The process backend
is a fallback with fewer capabilities; see [Compatibility](compatibility.md).
Install the release's [Yocto host requirements](https://docs.yoctoproject.org/brief-yoctoprojectqs/index.html)
separately; Yoctui does not install BitBake, BSP layers, or target tools.

## Understand the persistent shell

### Daemon management

Open **Tools → Daemons** in the navigator or **F12 → Tools → Daemons**.
The single local daemon has two views: `1` health/activity and `2` debug logs.
Health shows systemd status, PID, hostname/IP addresses, IPC state, startup
readiness, active jobs/terminals, memory, uptime and queue-pressure counters.
Logs are the latest 200 lines of the daemon's user-service journal, not the
BitBake task log. Refresh uses `r`; arrows, Page Up/Down and Home/End scroll.
Inspection refreshes in the background while the screen is open.

Use `s` start, `x` stop, `t` restart or `c` configure. Enter reviews/confirms;
Escape cancels. Stop/restart/reconfiguration refuses active jobs or terminal
sessions and disconnects clients when successful. Controls target only the local
`yoctui.service`; install it first with `yoctui daemon service install` if needed.
No remote/TCP daemon is supported here; the shown IPs identify the host only.

Configuration uses Tab to switch between the absolute build and source paths;
the build needs `conf/local.conf` and `conf/bblayers.conf`, and the source needs
`oe-init-build-env`. Clipboard paste is supported. Confirmation writes a separate
`yoctui.service.d/zz-yoctui-manager.conf` drop-in, reloads systemd and restarts the
daemon. It overrides any existing custom launch command without deleting it.
Vendor setup wrappers may need their own service configuration instead.
The source path is passed as `YOCTUI_SOURCE_DIR`, allowing sibling source/build
directories. Source trees, build outputs and Devtool workspaces are preserved.
The client reconnects automatically. Background compatibility discovery is
read-only, batches build-identity variables and reuses tool help output within
one discovery generation; it does not skip required evidence checks.

### Navigation

Navigator selects workspaces. Tab switches visible panes, Enter activates, and
Esc closes the innermost view. Ctrl+P searches commands; `a` opens context actions.
Inspector is off by default; Alt+i toggles it. Below 80x24, resize the terminal.
`q` requests exit; exiting does not cancel daemon-owned builds.

## Daily image-build loop

1. Check build directory, MACHINE, DISTRO, and compatibility.
2. Press F8 for Images, `1` for Artifacts, then `i` to select an image such as
   `core-image-minimal`; Enter accepts the selection. Tab focuses the workspace.
3. Press `b` to review the image build; final Enter starts it, Esc cancels the draft.
   For task/options selection, use Alt+b, then `e` to choose the target.
4. Follow Tasks, Logs (F5), and Errors. Cancel through the owning build controls.

Unknown totals stay unknown. Success, failure, cancellation, timeout, and loss
are distinct. `yoctui daemon build <target>` confirms submission, not image
completion; inspect status after an ambiguous connection failure before retrying.

## Core workspaces

| View | Purpose |
| --- | --- |
| Dashboard / History | State, telemetry, retained builds. |
| Tasks / Logs / Errors | Progress, output, exact diagnostic context. |
| Layers / Recipes / Devtool | Source browsing, editing, recipe-scoped work. |
| Configuration / BBMASK | Values, provenance, reviewed supported edits. |
| Images / Packages | Artifacts, packages, [rootfs inspection](rootfs-inspection.md). |
| Kernel / U-Boot / BIOS | [Sources, configuration, debugging](kernel-and-firmware.md). |
| Terminal Sessions | [Persistent shells, QEMU, SSH](terminal-sessions.md). |

## Offline use and saved builds

Without a daemon, local files/Git and saved history remain available; disconnected
views label data as last observed. F3 → select build → Enter opens Summary/Logs/Tasks/Errors.
`o` reviews loading its existing environment; active jobs/terminals prevent switching.
Loading refreshes metadata, not the historical build or MACHINE. Missing paths are not recreated.
History retains at most 32 builds/8 MiB under $XDG_STATE_HOME/yoctui/build-history
(default ~/.local/state/yoctui/build-history), not complete logs or running processes.

## Reading logs

F5 opens Logs, `f` follows, and `w` wraps; manual navigation pauses follow.
Context actions offer filters, search, bookmarks, source opens, and export.
Loss counters and `[viewport truncated]` explain retention/display limits.
Errors opens selected failure context; SSH/QEMU remain interactive terminals.

## Browse and edit layers, recipes, and configuration

### Layers

Enter browses a configured layer, `e` opens the editor, and `o` uses the external
editor. Arrows expand/collapse, `.` shows hidden files, and `r` refreshes.
Ctrl+S saves; conflicting disk changes block a save.

### Recipes and Devtool

Enter loads details; `b` reviews a recipe build, `f` chooses a task, `e` opens
its provider, `o` selects a task log, and `p` selects a patch. `A` opens Dependencies;
`Z` opens signature comparison. Tasks and paths require current metadata.

Devtool: `/` filters recipes, `t` refreshes status, `d` modifies/opens,
`u` creates patches, Alt+f finishes, Alt+p deploys, and Alt+d reviews reset.
Editor Ctrl+B refuses dirty content before recipe-build review.
Alt+w opens the selected source workspace; save/build, commit source changes,
then Alt+f reviews finishing the patch into a layer. Inspect it and rebuild.
Include the appropriate `Upstream-Status` in source commit messages before
exporting patches; Yocto's patch QA may reject patches without it.
Attached clients use the daemon's validated tool paths for status refresh;
modify, patch update and finish refresh automatically when their new job completes.
Managed GitUI needs writer control (Ctrl+B o); Ctrl+B e returns to the retained editor.

### Source Git

Header status shows branch, staged/unstaged/untracked/conflicts and ahead/behind
against the last fetched upstream; synced does not mean a clean worktree.
Status refresh never fetches. Install [GitUI](https://github.com/gitui-org/gitui#installation)
on PATH and reopen Yoctui. Alt+g reviews repository/terminal choice. Without a daemon,
use the current or a supported detached terminal; embedded GitUI uses writer leases.

### Configuration and BBMASK

Enter inspects, `s` changes scope, `c` compares, and `o` opens the reported source.
Only allowlisted global edits can write, after exact assignment/destination review.
BBMASK edits also require preview and confirmation.

## Dependency, package, and signature evidence

Dependencies: Enter opens the recipe, `o` its provider, `L` a task log, `r` refreshes.
Packages requires generated pkgdata; `R` refreshes and `D` changes dependency direction.
Signatures requires exact artifacts: choose sides with `1`/`2`, then `c` compares.

Overview → Insights offers timeline, rebuild causes, cache outcomes, size deltas,
provenance, package topology, supply-chain reports, and disk history (`1`–`8`).
Missing evidence is not estimated.

## Image, SDK, QEMU, and Wic operations

Images: `i` chooses a target, `b` reviews a build, `R` rescans, `o` opens an artifact.
`T` opens [QEMU/SSH consoles](terminal-sessions.md#image-consoles); `Q` offers advanced
QEMU review. `W` reviews Wic creation. Device writing (`D`) requires an eligible
removable device, the exact `WRITE <device-path>` phrase, and another confirmation.

SDK: `s`/`E` review standard/extensible builds; `t`/`T` tests; `R` rescan;
`P` publishing; `n` native-tool operation; `c` owned-job cancellation.

## Testing, Security, and QA

Testing launches supported tests, imports exact result paths, compares fingerprints,
and exports JUnit. Security inspects CVE/SBOM; QA offers recipe/kernel/layer checks.
Requests use typed review and owned cancellation; missing/partial results stay explicit.
Raw Mode provides a structured command catalog, argument reviews, favorites, and history.

## Maintenance

`[`/`]` selects Sstate/Services/Release/Integrations; `r` refreshes, `x` cancels.
Cleanup requires exact candidates, a deletion phrase, and confirmation.
Services/Integrations provide observations; Release mutations use review,
with a separate confirmation for network push.

## Background jobs, cancellation, and terminal outcomes

Navigate while jobs run; return to their workspace for output/outcome. Request
cancellation there and wait for acknowledgement. Lost is not cancelled or successful;
inspect external state before retrying.

## View and edit Hardware text files

Readable text of any extension, including extensionless files, opens from
Content → Hardware with Enter/`e` (1 MiB limit); `i` inserts, Ctrl+F searches,
Ctrl+S saves. Binary files cannot be text-edited. Saves preserve permissions and
reject conflicts, symlinks, and paths outside the project.
See [Hardware projects](hardware-projects.md) for imports, schematics, and manual bring-up.

## View Hardware PDFs

Select a PDF and press Enter. Native pages need SIXEL and document tools;
Yoctui can hand off to a suitable XTerm. Click the viewer toolbar, or use `=`/`+`
and `-` for centered zoom, `0` for fit-to-page and `w` for fit-to-width.
Arrows or mouse dragging pan; the wheel pans a magnified page (changes pages
when fitted), and Ctrl+wheel zooms. PgUp/PgDn always change page; `v` shows
embedded text and Esc returns. Press `o` to open the PDF in your desktop's
registered reader (GLib `gio` and, for example, Papers, Evince or Okular).
Its separate window stays open when you close Yoctui. This gives
full-resolution reading, selection, printing and the reader's available tools;
the embedded terminal raster viewer is not a PDF editor. Missing prerequisites are explained.

## Application menu

F12: Workspace, Build, Actions, Navigate, Config, View, Devtool, Tools, Help.
Arrows move, typing selects a prefix, Enter opens, Esc closes. BBMASK is in
Config; Preferences is first in View. Disabled actions explain why.

## Settings, configuration, and sessions

F12 → View → Preferences shows current/default/custom values. Arrows/Enter change,
Backspace resets one, Alt+r resets all; `r` retries a failed save to `session.toml`.
“Inspector at startup” saves the default; Alt+i changes only the current session.

Settings cover appearance, density, motion, mouse, logs, charts, pane restoration,
preview policy, and [keybindings](keyboard-shortcuts.md#customize-safely). CLI choices override
environment/config/session defaults; `--no-color` preserves the saved setting.
[Team profiles](terminal-sessions.md#team-profiles) share supported presets/workflows.

## Troubleshooting

| Problem | Check |
| --- | --- |
| Wrong workspace or old binary | `type -a yoctui`, `yoctui --version`, daemon status; restart only after work finishes. |
| Unknown release/empty inventory | Initialized shell, build directory, `yoctui doctor`. |
| Disabled action | Its tool/task/artifact/configuration prerequisite. |
| Missing packages/filesystem | Packaging/image tasks; rm_work can remove the logical rootfs. |
| Lost/failed job | Retained output and external state before retrying. |
| Damaged terminal after child exit | `reset`, `stty sane`, EDITOR configuration. |

## Reference boundaries

Fixtures illustrate behavior; they do not certify boot or every live workflow.
See [Compatibility](compatibility.md), [tools](tools-and-workflows.md), and [all guides](README.md).
