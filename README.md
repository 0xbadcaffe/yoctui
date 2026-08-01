# Yoctui

Yoctui is a Rust/Ratatui control frontend for Yocto/BitBake. BitBake remains the metadata and build authority; Yoctui observes it and requests operations.

Start with the installation and environment quickstarts below, then use the
[Operator Guide](docs/operator-guide.md) for daily builds, metadata editing,
Devtool, artifacts, testing, maintenance, and troubleshooting.

## Prerequisites and installation

Yoctui's production path targets a UTF-8 Linux terminal. Building Yoctui requires Git, Python 3, and a current stable Rust toolchain with Cargo. A real image build also needs the host packages required by the selected Yocto release; follow that release's Yocto Project Quick Build documentation rather than treating Yoctui's Rust prerequisites as a complete Yocto host setup.

Check the tools, clone Yoctui, and build both debug and release binaries:

```sh
export YOCTUI_DIR="$HOME/projects/yoctui"

command -v git
command -v python3
command -v rustc
command -v cargo
rustc --version

git clone https://github.com/0xbadcaffe/yoctui.git "$YOCTUI_DIR"
cd "$YOCTUI_DIR"
cargo build --locked -p yoctui
cargo build --locked --release -p yoctui
```

If `rustc` or `cargo` is missing, install stable Rust with the official rustup installer, restart the shell or source its printed environment file, and rerun the checks above. The binaries are `target/debug/yoctui` and `target/release/yoctui`. To install the release binary in Cargo's executable directory as `yoctui`:

```sh
cd "$YOCTUI_DIR"
cargo install --locked --path crates/yoctui-cli
command -v yoctui
yoctui --help
```

## Quickstart: current Yocto development setup

A checkout of Poky's `master` migration repository may contain only `README`; it does not provide `oe-init-build-env`. For current Yocto development, use BitBake's `bitbake-setup` workflow, which creates a setup from separate BitBake, OpenEmbedded-Core, and metadata repositories. This copyable block creates a `qemux86-64` setup and launches Yoctui without requesting a build:

```sh
export YOCTUI_DIR="$HOME/projects/yoctui"
export BITBAKE_DIR="$HOME/src/bitbake"
export YOCTUI_SETUP="yoctui-qemux86-64"

test -d "$YOCTUI_DIR/.git" || git clone https://github.com/0xbadcaffe/yoctui.git "$YOCTUI_DIR"
test -d "$BITBAKE_DIR/.git" || git clone https://git.openembedded.org/bitbake "$BITBAKE_DIR"
test -x "$BITBAKE_DIR/bin/bitbake-setup" || {
    echo "missing $BITBAKE_DIR/bin/bitbake-setup" >&2
    exit 1
}

cd "$BITBAKE_DIR"
./bin/bitbake-setup init --setup-dir-name "$YOCTUI_SETUP"

# In the interactive prompts choose the current poky-master template,
# the poky distro, and the qemux86-64 machine.
export YOCTUI_INIT="$BITBAKE_DIR/bitbake-builds/$YOCTUI_SETUP/build/init-build-env"
test -f "$YOCTUI_INIT" || {
    echo "missing $YOCTUI_INIT; review the bitbake-setup result" >&2
    exit 1
}
source "$YOCTUI_INIT"
test -n "${BUILDDIR:-}" || {
    echo "init-build-env did not export BUILDDIR" >&2
    exit 1
}

cd "$YOCTUI_DIR"
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR"
```

Inside Yoctui, press `B` for image build options, press `e`, enter `core-image-minimal`, then choose the build action and explicitly confirm it. Do not run `bitbake core-image-minimal` during setup: the first image build begins from this TUI confirmation. Build progress, package progress, CPU utilization, disk free space, and logs remain visible throughout the build.

Reopen the setup later with the same guards and no positional target:

```sh
export YOCTUI_DIR="$HOME/projects/yoctui"
export YOCTUI_INIT="$HOME/src/bitbake/bitbake-builds/yoctui-qemux86-64/build/init-build-env"
test -f "$YOCTUI_INIT" || { echo "missing $YOCTUI_INIT" >&2; exit 1; }
source "$YOCTUI_INIT"
test -n "${BUILDDIR:-}" || { echo "BUILDDIR is not set" >&2; exit 1; }
cd "$YOCTUI_DIR"
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR"
```

Yoctui never starts a build merely because the workspace opens. Press `!` to open an inherited Yocto shell for optional commands such as `bitbake-layers show-layers`; type `exit` to return.

## Quickstart: existing Poky checkout

Use this path only when `YOCTO_DIR` is a complete Poky release checkout that contains `oe-init-build-env`. Select the stable branch appropriate for the product before running this block; do not substitute the README-only Poky `master` migration repository.

```sh
export YOCTUI_DIR="$HOME/projects/yoctui"
export YOCTO_DIR="$HOME/src/poky"
export YOCTUI_BUILD_DIR="build-yoctui"

test -f "$YOCTO_DIR/oe-init-build-env" || {
    echo "missing $YOCTO_DIR/oe-init-build-env; use a complete Poky release or bitbake-setup" >&2
    exit 1
}
source "$YOCTO_DIR/oe-init-build-env" "$YOCTUI_BUILD_DIR"
test -n "${BUILDDIR:-}" || {
    echo "oe-init-build-env did not export BUILDDIR" >&2
    exit 1
}

cd "$YOCTUI_DIR"
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR"
```

Select and confirm `core-image-minimal` from `B` inside Yoctui exactly as in the current-development quickstart.

## Build

From the repository root:

```sh
cargo build -p yoctui
```

For an optimized binary:

```sh
cargo build --release -p yoctui
```

The binaries are written to `target/debug/yoctui` and `target/release/yoctui` respectively.

## Run

### Smoke checks without a Yocto checkout

This verifies the bundled bridge and terminal application from the Yoctui repository itself. It does not start a BitBake build:

```sh
cd ~/projects/yoctui
cargo build --locked -p yoctui
./scripts/headless-workload.sh target/debug/yoctui bridge
cargo run --locked -p yoctui -- doctor
```

The workload script gives the smoke run a temporary `XDG_CONFIG_HOME`, so a saved session cannot inject a remembered build directory or target. CI and other automation should use the same isolation pattern rather than reading a developer's configuration.

### Interactive UI with Yocto

Start from an initialized build directory. The following commands are intended to be copied as one block after choosing the correct complete Poky release checkout:

```sh
export YOCTO_DIR="$HOME/src/poky"
test -f "$YOCTO_DIR/oe-init-build-env" || { echo "missing $YOCTO_DIR/oe-init-build-env" >&2; exit 1; }
source "$YOCTO_DIR/oe-init-build-env" build
cd ~/projects/yoctui
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR"
```

If the build directory is already initialized in the current shell, start the UI directly:

```sh
cargo run --locked -p yoctui -- --backend bridge --build-dir "$BUILDDIR"
```

### Interactive cockpit shortcuts

Yoctui inherits the shell environment that initialized Yocto. Press `!` to temporarily leave the TUI for that shell, run commands such as `bitbake-layers show-layers`, `bitbake -e <target>`, or `bitbake <target>`, then run `exit` to return to Yoctui. The TUI restores after the shell ends.

Press `B` to open the image build-options submenu. It shows the effective `MACHINE` and current image target, then offers `b` to build, `c` to clean, `m` to run `menuconfig`, or `e` to enter a different image target. Press `y` for the Layers screen; every listed row is an active build layer and is highlighted green when color is enabled.

While BitBake is loading, parsing, running, or cancelling a build, the dashboard refreshes host CPU utilization and free space on the filesystem containing `$BUILDDIR` once per second.

The dashboard retains up to 1,024 completed package tasks for the current build alongside active tasks. Use `Up` and `Down` on the dashboard to scroll this package progress history; successful tasks are green and failed tasks red when color is enabled.

Press `h` to view up to 50 completed builds from the current Yoctui session, including target, result, exit code, elapsed time, package-task count, warnings, and errors.

The persistent header identifies the active Yocto release and source (or build) location. The bottom line changes with the current screen so its available shortcuts remain visible without opening Help.

Press `x` to inspect the effective `BBMASK` patterns and their backend-provided provenance. Press `e` to edit the intended value: Yoctui previews the exact assignment it will append to `$BUILDDIR/conf/local.conf`, requires confirmation, then refreshes workspace metadata.

In the Layers screen, select an active layer and press `e` to open its full metadata file tree in Yoctui’s two-pane editor. This is useful for layer recipes and configuration files; use `Enter` or `e` to edit a selected file, `Ctrl+S` to save, and `Esc` to return. Press `o` when you prefer the configured external editor.

Select a backend explicitly when needed:

```sh
# Versioned Python bridge (default)
cargo run -p yoctui -- --backend bridge --build-dir "$BUILDDIR"

# Direct bitbake process fallback; builds still require explicit TUI confirmation
cargo run -p yoctui -- --backend process --build-dir "$BUILDDIR"
```

The bridge backend is the default and provides versioned workspace inspection and typed live events when the active BitBake adapter supports them. The process backend directly invokes the inherited `bitbake` executable as a compatibility fallback and can expose fewer capabilities. Neither backend makes a mocked or environment-only bridge into live BitBake support; see the observed combinations in [Compatibility](docs/compatibility.md).

### Edit a recipe with Devtool

In the interactive Recipes screen, select a recipe and press `d`. Yoctui runs `devtool modify <recipe>` from the active build directory when needed, then opens a large in-TUI workspace editor for `$BUILDDIR/workspace/sources/<recipe>`. The left pane lists the workspace tree; the right pane displays the selected file. Use `Up`/`Down` to select a file, `Enter` or `e` to edit, `Ctrl+S` to save, and `Esc` to return to the main UI and build an image. Press `u` to run `devtool update-recipe <recipe>` after confirmation; Yoctui refreshes its workspace data after a successful update. Press `F` to finish a workspace into a destination layer: Yoctui prefills the providing layer when known, shows the full command, and requires confirmation. Press `P` to enter a deployment target and run confirmation-protected `devtool deploy-target <recipe> <target>`. Press `D` to reset that Devtool workspace; Yoctui requires confirmation before it runs `devtool reset <recipe>`. These are explicit user actions and require `devtool` from the initialized Yocto environment.

For scripting and CI, use a non-interactive workspace inspection. These commands work in an initialized Yocto shell; replace `"$BUILDDIR"` with an explicit path if it is not exported:

```sh
cargo run -p yoctui -- --headless --backend bridge --build-dir "$BUILDDIR"
cargo run -p yoctui -- --backend bridge --build-dir "$BUILDDIR" inspect
cargo run -p yoctui -- --backend bridge --build-dir "$BUILDDIR" recipes
cargo run -p yoctui -- --backend bridge --build-dir "$BUILDDIR" layers
```

Run diagnostics when environment setup is uncertain:

```sh
cargo run -p yoctui -- doctor
```

Use `cargo run -p yoctui -- --help` for the complete CLI reference.

The bridge protocol is NDJSON on standard I/O. The included bridge safely negotiates and inspects environment-derived workspace data without parsing configuration as authority; server operations require a compatible live BitBake adapter. See the complete [UI behavior reference](docs/ui-spec.md), [architecture](docs/architecture.md), [protocol](docs/protocol.md), [testing](docs/testing.md), [profiling](docs/profiling.md), and [compatibility evidence](docs/compatibility.md).

In the Recipes screen, press `g` to inspect the selected recipe's build and runtime dependencies. Use Up/Down and Enter to open a dependency recipe that is present in the workspace. This view is intentionally available only when the active BitBake server supports the bridge's `get_dependencies` capability; Yoctui does not infer dependencies itself.

Configuration is read from `$XDG_CONFIG_HOME/yoctui/config.toml` (or `~/.config/yoctui/config.toml`). For startup/runtime fields, CLI flags override `YOCTUI_*` environment variables, which override the configuration file, then the most recent session and built-in defaults. Interactive visual/log preferences saved from Settings override configuration defaults on the next run; hard CLI overrides such as `--no-color` remain authoritative. Supported values include `backend`, `build_dir`, `log_retention_entries`, `log_retention_bytes`, `refresh_ms`, `default_target`, `editor`, `color`, `theme`, `animation_speed`, `reduced_motion`, and `cancellation_timeout_ms`.

Interactive sessions are stored beside the configuration file in `session.toml`. Yoctui restores the last target, screen, log filters, wrap/follow modes, theme, animation speed, reduced-motion and color preferences, selected backend, and up to ten recent existing build directories. Settings changes are applied immediately and saved atomically without rewriting `config.toml`. Deleting `session.toml` safely resets those preferences.

## Development checks

```sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-checkout.sh
```

The strict final gate is `./scripts/verify-completion.sh`; it names any missing optional security, coverage, or profiling tool instead of reporting a false success. See [Testing](docs/testing.md) and [Profiling](docs/profiling.md) for details.

## Current limitations

The bridge protocol and mocked server adapter are fully testable without Yocto. Live BitBake server control still requires validation against a supported Yocto/BitBake environment; see [docs/compatibility.md](docs/compatibility.md).
