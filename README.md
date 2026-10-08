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
  <a href="#license"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT license"></a>
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/user-guide.md"><img src="https://img.shields.io/badge/docs-guide-blue?style=flat-square" alt="User guide"></a>
  <a href="#install"><img src="https://img.shields.io/badge/platform-Linux-purple?style=flat-square&amp;logo=linux&amp;logoColor=white" alt="Linux platform"></a>
  <a href="#contributing"><img src="https://img.shields.io/badge/community-GitHub-green?style=flat-square&amp;logo=github" alt="Contributions welcome on GitHub"></a>
</p>

<p align="center">
  <a href="#documentation">Docs</a> ·
  <a href="#install">Install</a> ·
  <a href="#features">Features</a> ·
  <a href="#quickstart">Quickstart</a> ·
  <a href="https://github.com/0xbadcaffe/yoctui">GitHub</a> ·
  <a href="https://github.com/0xbadcaffe/yoctui/issues">Issues</a>
  <br>
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/screenshots.md">Screenshots</a> ·
  <a href="#contributing">Contributing</a> ·
  <a href="#license">License</a>
</p>
<!-- /yoctui-header -->

# Yoctui

A terminal workspace for Yocto developers: build images, inspect what goes into
them, edit sources, and debug bring-up without losing your build context.
A persistent daemon keeps builds and development terminals available when you
close the UI and reconnect later.

<p align="center">
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/07-idle-dashboard.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/07-idle-dashboard.png" width="1000" alt="Yoctui dashboard with workspace status, recent builds and CPU, RAM and filesystem usage"></a>
</p>

## Features

- **Build and diagnose:** dashboard and insights, live BitBake tasks, logs,
  warnings, errors, and saved build history.
- **Develop:** browse layers and recipes, use devtool and devshell, edit text
  with syntax highlighting, and open GitUI in persistent terminal sessions.
- **Inspect images:** artifacts, rootfs package composition with a pie chart
  and list, navigable files with permissions and ownership, systemd services,
  D-Bus, and udev rules.
- **Bring up hardware:** organize project documents and requirements, and
  record progress through bootloader, kernel, device tree, drivers, and rootfs.
- **Debug:** kernel and U-Boot menuconfig, DTS/DTB tools, QEMU boot,
  SSH access, and remote kernel debugging with GDB.

Available actions depend on your Yocto build, generated artifacts, and installed
tools. See the [screenshots and workflows](https://github.com/0xbadcaffe/yoctui/blob/master/docs/screenshots.md)
for a closer look.

## Install

Linux, an 80×24+ terminal, Python 3, a C compiler/linker, and
[stable Rust/Cargo](https://www.rust-lang.org/tools/install) are required.
Install the official release:

```bash
cargo install yoctui --locked -j 2
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
```

The published crate may lag the source repository. For the latest checkout,
follow [Build from source](https://github.com/0xbadcaffe/yoctui/blob/master/docs/development.md#build-from-source).
QEMU, GDB, GitUI, and document viewers are optional tools needed for their
respective workflows.

## Quickstart

Use a shell initialized for your Yocto build
([workspace setup](https://github.com/0xbadcaffe/yoctui/blob/master/docs/user-guide.md#start-a-workspace-safely)).
Check the daemon's workspace first:

```bash
yoctui daemon status
```

If no daemon is running, or it already uses this build:

```bash
yoctui daemon start
yoctui --backend bridge --build-dir "$BUILDDIR"
```

To reconnect to an already-running daemon:

```bash
yoctui attach
```

Press `Alt+b` to choose/review a build, `F1` for Help, or `F12` for the menu.
Opening Yoctui does not start a build; exiting leaves daemon-owned work running.

## Documentation

[User guide](https://github.com/0xbadcaffe/yoctui/blob/master/docs/user-guide.md) ·
[Keyboard shortcuts](https://github.com/0xbadcaffe/yoctui/blob/master/docs/keyboard-shortcuts.md) ·
[Screenshots](https://github.com/0xbadcaffe/yoctui/blob/master/docs/screenshots.md) ·
[All documentation](https://github.com/0xbadcaffe/yoctui/blob/master/docs/README.md)

[Build from source](https://github.com/0xbadcaffe/yoctui/blob/master/docs/development.md#build-from-source) ·
[Profiling and Flamegraph](https://github.com/0xbadcaffe/yoctui/blob/master/docs/profiling.md#flamegraph) ·
[Issues](https://github.com/0xbadcaffe/yoctui/issues) ·
[Third-party notices](https://github.com/0xbadcaffe/yoctui/blob/master/docs/compliance/THIRD_PARTY_NOTICES.md)

## Contributing

Contributions are welcome, whether you're fixing your first typo or bringing
experience from a new board or Yocto release. Bug reports, documentation,
workflow feedback, tests, and code all help make Yoctui better.

[Open an issue](https://github.com/0xbadcaffe/yoctui/issues) with reproduction
steps and your Yocto/build environment, or send a focused
[pull request](https://github.com/0xbadcaffe/yoctui/pulls).
For larger changes, please discuss the approach in an issue first.
The [development guide](https://github.com/0xbadcaffe/yoctui/blob/master/docs/development.md)
covers source builds and verification.

## License

Yoctui is licensed under the [MIT License](https://github.com/0xbadcaffe/yoctui/blob/master/LICENSE).
Copyright © 2026 Roy Cohen. Third-party components retain their own licenses;
see [third-party notices](https://github.com/0xbadcaffe/yoctui/blob/master/docs/compliance/THIRD_PARTY_NOTICES.md).
