<!-- yoctui-header -->
<p align="center">
  <img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/yoctui-header.png" width="1000" alt="Yoctui — terminal interface for Yocto and BitBake development.">
</p>

<p align="center">
  <a href="https://github.com/0xbadcaffe/yoctui/actions/workflows/ci.yml"><img src="https://github.com/0xbadcaffe/yoctui/actions/workflows/ci.yml/badge.svg?branch=master" alt="CI workflow status"></a>
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/testing.md#completion-gate"><img src="https://img.shields.io/badge/coverage-gates-orange?style=flat-square" alt="Coverage verification gates"></a>
  <a href="https://crates.io/crates/yoctui"><img src="https://img.shields.io/crates/v/yoctui?style=flat-square&amp;cacheSeconds=300&amp;release=0.1.315" alt="Latest published crates.io version"></a>
  <a href="#install"><img src="https://img.shields.io/badge/rust-stable-orange?style=flat-square&amp;logo=rust" alt="Rust stable toolchain"></a>
  <br>
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT license"></a>
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md"><img src="https://img.shields.io/badge/docs-guide-blue?style=flat-square" alt="Operator documentation"></a>
  <a href="#install"><img src="https://img.shields.io/badge/platform-Linux-purple?style=flat-square&amp;logo=linux&amp;logoColor=white" alt="Linux platform"></a>
  <a href="https://github.com/0xbadcaffe/yoctui/issues"><img src="https://img.shields.io/badge/community-GitHub-green?style=flat-square&amp;logo=github" alt="Questions and issues on GitHub"></a>
</p>

<p align="center">
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md">Docs</a> ·
  <a href="#install">Install</a> ·
  <a href="#features">Features</a> ·
  <a href="#quickstart-poky-build-environment">Quickstart</a> ·
  <a href="https://github.com/0xbadcaffe/yoctui">GitHub</a> ·
  <a href="https://github.com/0xbadcaffe/yoctui/issues">Issues</a>
</p>
<!-- /yoctui-header -->

# Yoctui

Yoctui is a terminal application for Yocto and BitBake development. It runs
builds, shows tasks and logs, edits recipes and sources, inspects generated
images, and manages development terminals.

<!-- yoctui-contents -->
[Install](#install) · [Build from source](#build-from-source) ·
[Poky quickstart](#quickstart-poky-build-environment) · [Navigation](#navigation) ·
[Screenshots](#screenshots)

[Builds and logs](#build-logs-and-errors) · [Saved builds](#offline-use-and-saved-builds) ·
[Recipes and Devtool](#edit-recipes-and-develop-a-patch) · [GitUI](#source-git-status) ·
[Search](#search-source-code-and-generated-content)

[Kernel debugging and Insights](#kernel-firmware-and-build-analysis) ·
[Images and rootfs](#inspect-an-image-its-packages-and-rootfs) ·
[QEMU and SSH](#boot-with-qemu-or-connect-over-ssh) ·
[SDK, Wic and validation](#sdk-wic-tests-security-and-maintenance)

[Daemon and remote use](#daemon-and-remote-use) · [Settings and profiles](#settings-and-team-profiles) ·
[Features](#features) · [Troubleshooting](#compatibility-and-troubleshooting) ·
[Performance](#performance-evidence) · [Hardware bring-up](#hardware-projects-and-manual-bring-up) ·
[Development and license](#development-and-license) · [Operator guide](https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md)
<!-- /yoctui-contents -->

<p align="center">
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/07-idle-dashboard.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/07-idle-dashboard.png" alt="Yoctui dashboard with workspace status, recent builds and CPU, RAM and filesystem usage"></a>
</p>

The dashboard shows workspace status, recent jobs, build actions and host usage.
CPU, RAM and filesystem bars include percentages and capacity values.

## Install

Use Linux, a terminal of at least 80×24, Python 3, a C compiler/linker and a
recent stable Rust/Cargo ([Rust setup](https://www.rust-lang.org/tools/install)).
Install your chosen release's [Yocto host requirements](https://docs.yoctoproject.org/brief-yoctoprojectqs/index.html)
separately; Yoctui does not install BitBake, BSP layers or target tools.

Install the latest **published** release (which may lag the source repository):

```bash
cargo install yoctui --locked -j 2
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
yoctui --version
yoctui --help
```

### Build from source

For the latest repository code, install Git and clone into a new directory:

```bash
mkdir -p "$HOME/projects"
export YOCTUI_DIR="$HOME/projects/yoctui"
git clone https://github.com/0xbadcaffe/yoctui.git "$YOCTUI_DIR"
cd "$YOCTUI_DIR"
cargo build --release --locked -p yoctui --bin yoctui -j 2
# Optional: install this checkout's optimized binary on PATH.
cargo install --locked --path crates/yoctui-cli --force --bin yoctui -j 2
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
type -a yoctui
yoctui --version
```

The build output is `target/release/yoctui` unless Cargo's target directory is
overridden. `cargo install` uses release mode by default. These examples limit
**Cargo** to two compile jobs; they do not change BitBake/make parallelism.
Persist the PATH setting in your shell configuration if needed (custom Cargo
install roots use their own `bin` directory).

An already-running client or daemon keeps its old executable. Reopen the client
after updating; restart the daemon **only after its builds and sessions finish**.

## Quickstart: Poky build environment

In **Bash**, use an existing complete Poky checkout; replace the paths below:

```bash
export POKY_DIR="$HOME/src/poky"
export BUILDDIR="$POKY_DIR/build-yoctui"
test -f "$POKY_DIR/oe-init-build-env" || {
  echo "Missing oe-init-build-env; check the source layout."
  exit 1
}
source "$POKY_DIR/oe-init-build-env" "$BUILDDIR"
yoctui daemon status
```

If status reports a different workspace, stop here: finish its work before
switching. Otherwise start the daemon (or reuse the one for this build):

```bash
yoctui daemon start
yoctui --backend bridge --build-dir "$BUILDDIR"
```

Use the actual setup script for your checkout: split layouts may place it under
`layers/openembedded-core`; BSPs such as OpenBMC may require a vendor wrapper.
Run Yoctui as the build user, not root. Opening it does not start a build.

Start the daemon from this initialized shell. An existing daemon is **not**
retargeted by `daemon start`; check its workspace before attaching or switching.
For later visits, use `yoctui attach`.

To build in Poky, press `Alt+b`, edit the target with `e`, enter
`core-image-minimal`, then review and confirm. For a BSP, choose an image it
actually supports. Configure `MACHINE`, host dependencies and build parallelism
in your Yocto environment before building.

### Configure paths inside Yoctui

Without an environment, open **Build environment → e**. Tab selects
Source/Build/Script; `b` browses, `e` edits/pastes and `s` saves. Use an existing
build directory and the correct setup script. `Alt+v` initializes/verifies;
browsing and saving alone execute nothing. Persistent sessions still require
the daemon started from an initialized shell.
[Workspace setup details](https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md#start-a-workspace-safely).

## Navigation

| Key | Action |
| --- | --- |
| `F1` / `?` | Help |
| `F2` / `F3` / `F4` | Tasks / History / Dashboard |
| `F5` / `F6` / `F7` / `F8` | Logs / Layers / Recipes / Images |
| `F9` / `Ctrl+P` | Command palette |
| `F12` / `a` | Application menu / context actions |
| `Tab` / `Shift+Tab` | Change focus; some workspaces use Tab for their views |
| Arrows, `PageUp`/`PageDown`, `Home`/`End` | Move within lists and trees |
| `Right` / `Enter` in Navigator | Expand a group, then open and focus its workspace |
| `Left` in a tree | Collapse or move to parent |
| `Esc` | Close the active context, return to Navigator, then Dashboard |
| `/` | Search build-file contents, generated rootfs and text image artifacts; opens empty (outside editors and local search fields) |
| `Alt+b` | Image build options |
| `q` | Request exit; confirmation required |

Use `Alt+w` to start/open a Recipes/Devtool workspace, `Alt+g` for GitUI, and
`Alt+f` for integrated-editor workspace search (`Ctrl+F` searches the file).
Uppercase application shortcuts have Alt-lowercase alternatives; literal text
and native terminal-program keys stay unchanged.

The footer lists shortcuts for the current view. Dialogs and editors own
their keys before global navigation. Terminal writers retain normal keys; use
`Ctrl+B` for Yoctui terminal controls. See the [keymap](https://github.com/0xbadcaffe/yoctui/blob/master/docs/keymap.md) for
terminal-prefix commands and custom bindings. `Ctrl+V` pastes into focused
text fields/editors when `wl-paste`, `xclip` or `xsel` is available; native
terminals keep their own paste/key behavior.

## Screenshots

Screens use fixture values rendered through Yoctui at `160x50`. GitUI panes
replay native output from a demo repository. For live build captures and image
checksums, see [Raster provenance](https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/manifest.toml),
[Recorded live capture](https://github.com/0xbadcaffe/yoctui/blob/master/artifacts/release-quality/next-generation-ui/manifest.json),
[Completed live build](https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/yoctui-live-completion.svg) and
[Failed live build](https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/yoctui-live-failed-task.svg).

### Set up a workspace

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/13-cloning.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/13-cloning.png" alt="Fresh clone"></a><br><strong>Clone sources</strong> — Background cloning with a Braille <code>Cloning…</code> indicator.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/18-offline-dashboard.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/18-offline-dashboard.png" alt="Offline Dashboard with saved builds and setup guidance"></a><br><strong>Offline dashboard</strong> — Configure paths, reconnect the daemon or open saved builds.</td>
  </tr>
</table>

### Build and debug

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/01-active-build-tasks.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/01-active-build-tasks.png" alt="Yoctui active BitBake tasks and correlated build logs"></a><br><strong>Tasks</strong> — Active BitBake tasks, build progress and task logs.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/08-failed-build-errors.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/08-failed-build-errors.png" alt="Yoctui failed BitBake task errors and correlated logs"></a><br><strong>Errors</strong> — Failed tasks, diagnostics, source logs and recovery actions.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/14-cancelling.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/14-cancelling.png" alt="Background cancellation"></a><br><strong>Cancel a build</strong> — Cancellation runs in the background; navigation stays available.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/15-search-empty.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/15-search-empty.png" alt="Content search"></a><br><strong>Search build output</strong> — <code>/</code> opens an empty search of build files, generated rootfs and text image artifacts.</td>
  </tr>
</table>

### Read previous builds

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/19-saved-build-history.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/19-saved-build-history.png" alt="Saved build history without a daemon connection"></a><br><strong>Build history</strong> — Browse saved outcomes without a configured environment or daemon connection.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/20-saved-build-logs.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/20-saved-build-logs.png" alt="Read-only saved build logs with provenance"></a><br><strong>Saved logs and tasks</strong> — Read retained build details and log excerpts; missing records and retention limits are shown.</td>
  </tr>
</table>

### Edit sources and commit changes

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/09-editor-application-menu.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/09-editor-application-menu.png" alt="Yoctui BitBake recipe editor and application action menu"></a><br><strong>Recipe editor and menus</strong> — Syntax highlighting, validation and context actions. Use arrows to navigate menus and Escape to return; unavailable actions show the reason.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/10-terminal-sessions.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/10-terminal-sessions.png" alt="Yoctui split daemon-owned terminal sessions"></a><br><strong>Terminal sessions</strong> — Split build shells and devshells with writer control and scrollback.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/16-gitui-diff.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/16-gitui-diff.png" alt="Source Git and diffs"></a><br><strong>Git status and diffs</strong> — Repository status in the header; native GitUI for reviewing and staging changes.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/17-gitui-commit.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/17-gitui-commit.png" alt="Commit messages"></a><br><strong>Commit changes</strong> — Enter commit messages in GitUI inside a Yoctui terminal.</td>
  </tr>
</table>

### Configure the kernel and bootloader

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/02-kernel-device-tree.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/02-kernel-device-tree.png" alt="Yoctui Kernel device-tree inventory"></a><br><strong>Kernel device trees</strong> — DTS, DTSI and compiled DTB files for the selected provider.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/03-uboot-device-tree.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/03-uboot-device-tree.png" alt="Yoctui U-Boot device-tree inventory"></a><br><strong>U-Boot device trees</strong> — Bootloader sources and generated device-tree artifacts.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/04-kernel-menuconfig.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/04-kernel-menuconfig.png" alt="Linux kernel menuconfig inside a Yoctui terminal session"></a><br><strong>Kernel menuconfig</strong> — Native ncurses controls in a daemon-owned terminal.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/05-uboot-menuconfig.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/05-uboot-menuconfig.png" alt="U-Boot menuconfig inside a Yoctui terminal session"></a><br><strong>U-Boot menuconfig</strong> — The selected provider’s ncurses interface in a reconnectable PTY.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/11-device-tree-editor.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/11-device-tree-editor.png" alt="Yoctui Device Tree source editor with DTS syntax highlighting"></a><br><strong>Device Tree editor</strong> — Linux v6.6 <a href="https://github.com/torvalds/linux/blob/v6.6/arch/arm64/boot/dts/freescale/imx8mp-evk.dts">NXP i.MX8MP EVK DTS</a>, with highlighted directives, nodes, properties, values and comments.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/12-device-tree-compile-options.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/12-device-tree-compile-options.png" alt="Yoctui dtc compile-options dialog for a kernel Device Tree source"></a><br><strong>Device Tree compiler</strong> — Set symbols, sorting, padding and reserve entries, then review the exact <code>dtc</code> command.</td>
  </tr>
</table>

### Inspect the generated image

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/06-rootfs-composition.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/06-rootfs-composition.png" alt="Yoctui root filesystem package composition pie chart and exact size table"></a><br><strong>Rootfs composition</strong> — Braille package-size chart with matching table colors, exact byte totals and filesystem drill-down.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/21-systemd-services.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/21-systemd-services.png" alt="Offline systemd Services view listing unit files, descriptions, BusName and enablement from IMAGE_ROOTFS"></a><br><strong>Offline systemd services</strong> — Inspect unit files, D-Bus names and enablement links in <code>IMAGE_ROOTFS</code>. These are installed files, not live service status.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/22-system-dbus.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/22-system-dbus.png" alt="Offline system D-Bus activation map with bus names, systemd units, users, executables and policy counts"></a><br><strong>System D-Bus</strong> — Inspect activation files, associated systemd units and policy files from the image.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/23-udev-rules.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/23-udev-rules.png" alt="Offline udev rule files showing overrides, masks and selected rule content"></a><br><strong>udev rules</strong> — Check file precedence, overrides and masks, then read the selected rule. Rules are not executed.</td>
  </tr>
</table>

## Build, logs and errors

1. Use `Alt+b` for an image build, or `b` on a selected recipe.
2. Open Tasks with `F2`; use its filters to inspect active or failed tasks.
3. Open Logs with `F5`. `f` resumes live follow; navigation pauses follow.
   `w` toggles wrapping. Context actions expose local search and filters.
4. Open Errors from the Navigator to inspect failures and their source logs.
5. Use the cancel action and confirm it to stop a build. Exiting the client is
   separate from cancelling a daemon-owned build.

Logs show output acquired by Yoctui, not every log file on the host.
Retention is bounded. [Log controls and limits](https://github.com/0xbadcaffe/yoctui/blob/master/docs/yocto-logs.md).

## Offline use and saved builds

Without a daemon, configure paths, browse saved history and work with local
files/Git. Disconnected screens label retained data as last observed.

Press `F3`, select a build and Enter for Summary/Logs/Tasks/Errors. Press `o`
to **Load environment**, review the existing source/build paths and confirm.
Missing paths are not recreated; active jobs or terminals prevent switching.
Loading refreshes metadata, not the historical build, and does not change MACHINE.

Saved history contains bounded excerpts (32 builds, 8 MiB), not complete logs
or proof that an interrupted process is still running. It lives under
`$XDG_STATE_HOME/yoctui/build-history` (default `~/.local/state/yoctui/build-history`).
[History and connection details](https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md#understand-the-persistent-shell).

## Edit recipes and develop a patch

Layers provides an expandable file tree; `e` opens the built-in editor.
In Recipes, select a provider and use this reviewed patch workflow:

1. `e` edits the recipe; `t` refreshes Devtool status and `Alt+w` starts/opens
   its source workspace.
2. Edit with syntax highlighting, search and undo/redo; `Ctrl+S` saves.
3. From a saved recipe/source editor, `Ctrl+B` reviews that recipe's build.
4. `u` updates the recipe; commit the source changes, then `Alt+f` in Recipes
   reviews `devtool finish` into a layer. Inspect the patch and rebuild.

Devshell/menuconfig can use embedded or supported detached terminals.
The editor is Vim-style, not an LSP/VS Code replacement.
[Editing, deploy and reset controls](https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md#recipes-and-devtool).

## Source Git status

The header shows branch, staged/unstaged/untracked/conflicted counts and
ahead/behind against the last fetched upstream. `synced*` does not mean a clean
worktree; background status refresh never fetches.

Install [GitUI](https://github.com/gitui-org/gitui#installation) on PATH and reopen
Yoctui. `Alt+g` or **F12 → Tools → Open GitUI** reviews the repository and
destination. For embedded GitUI, focus its pane and press `o` for writer control;
its native keys then work in the full workspace. `Ctrl+B e` returns to a retained
editor without ending GitUI. A source repository is enough for GitUI; without a
daemon, choose the current terminal or a supported detached terminal.

## Search source code and generated content

Press `/` for actions and case-insensitive regex content search, such as
`systemd|udev`, across configured sources, build metadata/logs, retained rootfs
and text artifacts. Enter opens a result; provenance identifies its file/image.
Search skips symlinks, binary/oversized files and large caches, with at most
500 hits. It is not an exhaustive disk index. Editors use `Ctrl+F` for the file
and `Alt+f` for workspace search; native terminals keep their own keys.
[Search scope and controls](https://github.com/0xbadcaffe/yoctui/blob/master/docs/keymap.md#focus-and-collection-movement).

## Kernel, firmware and build analysis

In **Kernel** or **U-Boot / BIOS**, `m` opens native menuconfig; `e` edits files,
`c` reviews DTS compilation and `d` reviews DTB/DTBO decompilation (`dtc` required).
Generated `.yoctui` outputs refuse overwrites.

**Kernel → 3 Debugging** (`b`) includes managed **QEMU → GDB**, remote GDB,
serial KGDB and diagnostic tools such as strace, perf, ftrace and trace-cmd.
Debug fields discover selected-build defaults without overwriting edits;
missing/ambiguous files remain explicit. Supply matching uncompressed `vmlinux`
with debug symbols (possibly under `boot/.debug`), not a stripped boot image.
Review the exact command before launch. Runtime SSH tools need target permissions
and kernel support; **Host** mode means the Yoctui host, not the guest.

Serial KGDB requires an already configured/halted board; setup guides do not
automatically configure, flash, reset or halt hardware.
[Kernel/debugging prerequisites and workflows](https://github.com/0xbadcaffe/yoctui/blob/master/docs/platform-workbenches.md).

**Overview → Insights** offers timeline, rebuild, cache, size, provenance,
dependency, supply-chain and disk views (`1`–`8`). Missing evidence is not estimated.
Use Dependencies, Configuration and recipe signature history for deeper analysis.
[Analysis controls](https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md#dependency-package-and-signature-evidence).

## Inspect an image, its packages and rootfs

Open Images with `F8`. Select a deployed artifact, press `Alt+r` to rescan if
needed, and use the numbered views:

| Key | View |
| --- | --- |
| `1` | Deployed artifacts |
| `2` | Installed packages and size pie chart |
| `3` | Rootfs filesystem |
| `4` | Systemd services |
| `5` | System D-Bus |
| `6` | udev rules |

Artifacts show size and modification time; `o` opens supported text/DTS or
reviews DTB decompilation. Composition pairs the pie chart with an exact table
(table-only in narrow/accessibility modes).

Packages need the image manifest/pkgdata; files, services, D-Bus and udev need
retained `IMAGE_ROOTFS`. Yoctui does not mount/extract ext4/Wic automatically;
`rm_work` may remove the tree. The filesystem shows target permissions/owners,
not host build-user ownership. Services/rules describe installed files, not live
state, and are not executed. Make lasting changes in recipes/layers, not staged
rootfs files. [Rootfs evidence and controls](https://github.com/0xbadcaffe/yoctui/blob/master/docs/rootfs-composition.md).

## Boot with QEMU or connect over SSH

With an image artifact selected, press `Alt+t` in Images:

- **Boot with QEMU:** review the image and options. The embedded console uses
  `runqemu` with `nographic` and `serialstdio`.
- **Connect over SSH:** enter the host, user, port and optional identity file.
  This connects to a running target; it does not boot it. OpenSSH's host-key
  checks remain enabled.

Confirming creates a daemon-owned Terminal Session. `Ctrl+B`, then `o`,
takes writer control. `Alt+q` opens the advanced QEMU options.

| Prefix sequence | Action |
| --- | --- |
| `Ctrl+B c` | Create a build shell |
| `Ctrl+B n` / `Ctrl+B p` | Next / previous session |
| `Ctrl+B %` / `Ctrl+B "` | Split panes |
| `Ctrl+B z` | Zoom pane |
| `Ctrl+B [` / `Ctrl+B /` | Copy / search |
| `Ctrl+B d` | Detach client; keep the process |
| `Ctrl+B Alt+k` | Confirm process termination |
| `Ctrl+B ?` | Prefix help |

Press the prefix and its command separately. `!` opens an inherited shell
outside the TUI; exit that shell to return.
[Terminal sessions](https://github.com/0xbadcaffe/yoctui/blob/master/docs/embedded-shell.md).

## SDK, Wic, tests, security and maintenance

Use the Navigator for SDK builds/tests/installers, Wic image creation, selftests
and ptest, CVE/SBOM imports, QA checks, structured Raw Mode commands and maintenance.
Actions depend on the active image/machine/distro and show prerequisites/review.
Cleanup and removable-device writing require explicit confirmation; Yoctui
does not invoke sudo for Wic writes. A package manifest is not a full SBOM.
[Workflow controls and safeguards](https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md#image-sdk-qemu-and-wic-operations).

## Daemon and remote use

```bash
yoctui daemon status
yoctui attach
yoctui sessions
# Submit a supported image; submission is not proof of build completion.
yoctui daemon build core-image-minimal
```

The daemon keeps jobs and PTYs alive when a client disconnects. Connect to the
build host over SSH, then run `yoctui attach` there; the daemon uses a local
per-user Unix socket, not a public TCP listener.

Only after builds and sessions finish, use `yoctui daemon restart` to load a new
binary, or `yoctui daemon stop`. Reboot does not resume child processes: saved
metadata reports lost work as Lost, not running or successful.

Optional systemd user-service setup:

```bash
yoctui daemon service install
yoctui daemon service status
```

Arrange the correct Yocto environment before starting the service; unit
installation alone does not initialize a build or guarantee reboot readiness.
For failures, inspect `yoctui daemon status` and
`$XDG_STATE_HOME/yoctui/daemon.log` (default `~/.local/state/yoctui/daemon.log`).
[Daemon/session lifecycle](https://github.com/0xbadcaffe/yoctui/blob/master/docs/embedded-shell.md#daemon-owned-terminal-sessions).

## Settings and team profiles

Settings controls themes, mouse, reduced motion, ASCII/no-color and keybindings.
Preferences are local. Optional `.yoctui/project.toml` profiles share favorites,
build presets and workflows, not credentials/host paths/shell hooks.
`yoctui --build-dir "$BUILDDIR" profile` inspects one without executing it;
select a preset and review before running.
[Settings/profile reference](https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md#settings-configuration-and-sessions).

## Features

| Area | Features |
| --- | --- |
| Build environment | Source/build directory browser, manual path editing, environment-script detection, nested fresh-clone destinations, background cloning with Braille progress, initialization and connection checks |
| Dashboard and Tasks | Build/task state, progress, elapsed time, job history, background cancellation, CPU/RAM/filesystem bars, disk/network histories |
| Logs and Errors | Live follow/pause, filters, search, bookmarks, wrapping, horizontal scrolling, copy/export, task correlation and failure details; Yocto log panes use tui-logger |
| Layers and Recipes | Expandable layer tree using tui-tree-widget, provider/appends/tasks/patch inspection, syntax-aware file previews, source editing and external editors |
| Configuration | Effective values, overrides, provenance, scope comparison, reviewed local.conf edits and BBMASK |
| Dependencies and signatures | Recipe/task graphs, runtime dependencies, reverse traversal, why-built paths, signature inspection and diffsigs comparison |
| Devtool | Status, modify, source editing, recipe builds, update-recipe, finish into a layer, deploy and reset |
| Packages and Images | Generated pkgdata, installed packages, deployed artifacts, rootfs package pie chart using tui-piechart, filesystem tree, systemd units, system D-Bus configuration and udev rules |
| Kernel and firmware | Provider/configuration discovery, native menuconfig, DTS/DTB tools, managed QEMU→GDB, remote/serial debugging and reviewed diagnostics |
| Hardware bring-up | Persistent projects/documents, editable syntax-aware text, PDF/schematic viewers and manual milestone progress |
| Overview Insights | Timeline/critical path, rebuild causes, sstate/download outcomes, image size and retained size deltas, metadata provenance, package topology, supply-chain reports and disk history |
| Offline history | Saved build outcomes, machine, duration, bounded logs/tasks and missing-record notices; no daemon required |
| Source Git | Global branch, staged/unstaged/untracked/conflict and ahead/behind status; embedded GitUI diffs, staging and commits |
| Terminals | Daemon-owned shells, devshell/menuconfig, SSH and runqemu consoles using tui-term; split panes, resize, scrollback, copy/search and reconnect |
| SDK and Wic | Standard/extensible SDK builds and tests, installer inspection/publication, native tools, Wic creation and confirmed removable-device writing |
| Testing, Security and QA | Selftests, image/SDK tests, ptest, result comparison/JUnit export, CVE checks, SPDX/CycloneDX/manifest imports, recipe/kernel and layer checks |
| Raw Mode and Maintenance | Structured command catalog, argument previews, favorites/history, sstate checks/cleanup, PR/hash diagnostics, locked caches, buildhistory comparison and Git archives |
| Preferences and profiles | Color themes, reduced motion, ASCII/no-color views, keybindings, saved preferences, onboarding and optional team project profiles |

Actions depend on the connected Yocto environment and generated files.
Unavailable actions show the required tools, tasks or files.

## Compatibility and troubleshooting

Yoctui functionality is Yocto-feature-correlated: available actions depend on
what the initialized Yocto release and BSP provide.

Open Compatibility for detected tools/tasks/versions and disabled-action reasons:

```sh
yoctui --build-dir "$BUILDDIR" doctor
yoctui --build-dir "$BUILDDIR" doctor --json
```

The [compatibility matrix](https://github.com/0xbadcaffe/yoctui/blob/master/docs/compatibility-matrix.md) records exact tested
Yocto/BitBake revisions and host limits. A version number alone is not support
evidence; workflows depend on the connected environment's actual capabilities.

| Symptom | Check |
| --- | --- |
| Daemon unavailable | Start it from the initialized Yocto shell; check `yoctui daemon status` |
| Wrong workspace or old version | Check `type -a yoctui`, `yoctui --version` and the daemon's build directory; restart only when work is stopped |
| Packages unavailable after a build | Select the correct image and refresh; confirm its manifest and `tmp/pkgdata` still exist |
| No rootfs tree/services/udev | Confirm the reported `IMAGE_ROOTFS` was not cleaned; deploy artifacts alone are insufficient |
| Logs stop following | Press `f` in Logs; navigation pauses follow |
| Disabled workflow | Read its Compatibility reason and verify the required tool/task/configuration |
| Host overloaded or nearly full | Inspect telemetry and disk space; choose build parallelism/cleanup yourself |

## Performance evidence

This Flamegraph records userspace `perf` samples from the deterministic
large-metadata workload at v0.1.64 on September 6, 2026. It completed 6,000
frames with 2,403 real userspace samples, workload checksum
`95d507f9b14b71d6`, and zero unresolved frames. Rendering and model code have
changed since this capture. The historical measurements apply to v0.1.64, not
the current release.

<p align="center">
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/artifacts/flamegraph/yoctui.svg"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/artifacts/flamegraph/yoctui.svg" alt="Interactive Yoctui large-metadata workbench CPU Flamegraph"></a>
</p>

[Machine-readable summary](https://github.com/0xbadcaffe/yoctui/blob/master/artifacts/flamegraph/summary.txt) ·
[Profiling method and current limits](https://github.com/0xbadcaffe/yoctui/blob/master/docs/profiling.md)

Reproduce the current-source report on a Linux host that permits userspace
`perf` sampling:

```sh
cargo install flamegraph --locked
./scripts/flamegraph.sh
./scripts/test-flamegraph.sh
```

## Hardware projects and manual bring-up

Open **Hardware → p** for Projects. `n` creates a project/subfolder; `a` imports
files without moving/overwriting their sources (256 MiB limit). Projects persist
under `$XDG_DATA_HOME/yoctui/hardware-projects` (default
`~/.local/share/yoctui/hardware-projects`).

Readable text of any extension, including extensionless files, opens in the
syntax-aware Vim-style editor; `Ctrl+S` saves with conflict/permission checks.
Unknown languages remain plain text (1 MiB editor limit). PDF and supported
schematics use graphical viewers; `e` explicitly edits readable source text.
KiCad needs `kicad-cli`; Altium/Xpedition need a same-stem PDF export for graphical
viewing. Stored files are not executed.

`s` edits manual 0–100% bring-up values for Bootloader, Kernel, Device tree,
Drivers, RootFS and Packages. Enter saves; Esc cancels. The overall bar averages
these entries, not build-task progress.
[Hardware text and viewer details](https://github.com/0xbadcaffe/yoctui/blob/master/docs/operator-guide.md#view-and-edit-hardware-text-files).

## Development and license

From the repository root, build optimized for interactive use as shown above.
Developer verification commands (not required to install/run):

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/test-readme-quickstart.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

[Testing](https://github.com/0xbadcaffe/yoctui/blob/master/docs/testing.md) · [Profiling](https://github.com/0xbadcaffe/yoctui/blob/master/docs/profiling.md) ·
[Performance contract](https://github.com/0xbadcaffe/yoctui/blob/master/docs/performance.md) · [UI specification](https://github.com/0xbadcaffe/yoctui/blob/master/docs/ui-spec.md) ·
[Architecture](https://github.com/0xbadcaffe/yoctui/blob/master/docs/architecture.md) · [Implementation status](https://github.com/0xbadcaffe/yoctui/blob/master/docs/implementation-status.md)

Yoctui is [MIT-licensed](https://github.com/0xbadcaffe/yoctui/blob/master/LICENSE). Dependency licenses are listed in
[third-party notices](https://github.com/0xbadcaffe/yoctui/blob/master/docs/compliance/THIRD_PARTY_NOTICES.md).
The offline systemd service view was informed by the MIT-licensed
[systemd-manager-tui](https://github.com/Matheus-git/systemd-manager-tui) by
Matheus-git; Yoctui uses its own parser for unbooted images.
