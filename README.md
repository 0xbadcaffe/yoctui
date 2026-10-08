<!-- yoctui-header -->
<p align="center">
  <img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/yoctui-header.png" width="800" alt="Yoctui — terminal interface for Yocto and BitBake development.">
</p>

[![CI](https://github.com/0xbadcaffe/yoctui/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/0xbadcaffe/yoctui/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/yoctui?cacheSeconds=300&release=0.1.315)](https://crates.io/crates/yoctui)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](https://github.com/0xbadcaffe/yoctui/blob/master/LICENSE)
<!-- /yoctui-header -->

# Yoctui

A terminal interface for Yocto and BitBake: run builds, follow tasks and logs,
edit recipes, inspect images, and manage persistent development terminals.
Yoctui functionality is Yocto-feature-correlated: actions depend on your build.

## Install

Linux, an 80×24+ terminal, Python 3, a C compiler/linker, and
[stable Rust/Cargo](https://www.rust-lang.org/tools/install) are required.
Install the official release:

```bash
cargo install yoctui --locked -j 2
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
```

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
