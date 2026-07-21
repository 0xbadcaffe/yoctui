# Yoctui

Yoctui is a Rust/Ratatui control frontend for Yocto/BitBake. BitBake remains the metadata and build authority; Yoctui observes it and requests operations.

## Prerequisites

- Stable Rust with Cargo.
- Python 3 for the bundled bridge.
- A separate Poky/Yocto checkout when using a real BitBake workspace. `oe-init-build-env` is **not** part of this repository.

  Set `YOCTO_DIR` to the directory that contains `oe-init-build-env`, then initialize its build environment:

  ```sh
  export YOCTO_DIR="$HOME/src/poky"
  test -f "$YOCTO_DIR/oe-init-build-env"
  source "$YOCTO_DIR/oe-init-build-env" build
  ```

The bridge backend is the default. The process backend invokes the `bitbake` executable as a compatible fallback.

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

### Quick start without a Yocto checkout

This verifies the bundled bridge and terminal application from the Yoctui repository itself. It does not start a BitBake build:

```sh
cd ~/projects/yoctui
cargo build -p yoctui
cargo run -p yoctui -- --headless --backend bridge --build-dir "$PWD"
```

### Interactive UI with Yocto

Start from an initialized build directory. The following commands are intended to be copied as one block after choosing the correct `YOCTO_DIR`:

```sh
export YOCTO_DIR="$HOME/src/poky"
source "$YOCTO_DIR/oe-init-build-env" build
cd ~/projects/yoctui
cargo run -p yoctui -- --build-dir "$BUILDDIR" core-image-minimal
```

If the build directory is already initialized in the current shell, start the UI directly:

```sh
cargo run -p yoctui -- --build-dir "$BUILDDIR" core-image-minimal
```

### Interactive cockpit shortcuts

Yoctui inherits the shell environment that initialized Yocto. Press `!` to temporarily leave the TUI for that shell, run commands such as `bitbake-layers show-layers`, `bitbake -e <target>`, or `bitbake <target>`, then run `exit` to return to Yoctui. The TUI restores after the shell ends.

Press `B` to open the image build-options submenu. It shows the effective `MACHINE` and current image target, then offers `b` to build, `c` to clean, `m` to run `menuconfig`, or `e` to enter a different image target. Press `y` for the Layers screen; every listed row is an active build layer and is highlighted green when color is enabled.

While BitBake is loading, parsing, running, or cancelling a build, the dashboard refreshes host CPU utilization and free space on the filesystem containing `$BUILDDIR` once per second.

The dashboard retains up to 1,024 completed package tasks for the current build alongside active tasks. Use `Up` and `Down` on the dashboard to scroll this package progress history; successful tasks are green and failed tasks red when color is enabled.

The persistent header identifies the active Yocto release and source (or build) location. The bottom line changes with the current screen so its available shortcuts remain visible without opening Help.

Press `x` to inspect the effective `BBMASK` patterns and their backend-provided provenance. Press `e` to edit the intended value: Yoctui previews the exact assignment it will append to `$BUILDDIR/conf/local.conf`, requires confirmation, then refreshes workspace metadata.

In the Layers screen, select an active layer and press `e` to open its full metadata file tree in Yoctui’s two-pane editor. This is useful for layer recipes and configuration files; use `Enter` or `e` to edit a selected file, `Ctrl+S` to save, and `Esc` to return. Press `o` when you prefer the configured external editor.

Select a backend explicitly when needed:

```sh
# Versioned Python bridge (default)
cargo run -p yoctui -- --backend bridge --build-dir "$BUILDDIR"

# Direct bitbake process fallback
cargo run -p yoctui -- --backend process --build-dir "$BUILDDIR" core-image-minimal
```

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

The bridge protocol is NDJSON on standard I/O. The included bridge safely negotiates and inspects environment-derived workspace data without parsing configuration as authority; server operations require a compatible live BitBake adapter. See `docs/` for architecture, testing, profiling, protocol, and compatibility details.

Configuration is read from `$XDG_CONFIG_HOME/yoctui/config.toml` (or `~/.config/yoctui/config.toml`). CLI flags override `YOCTUI_*` environment variables, which override the configuration file, which overrides the most recent session, which overrides built-in defaults. Supported values include `backend`, `build_dir`, `log_retention_entries`, `log_retention_bytes`, `refresh_ms`, `default_target`, `editor`, `color`, and `cancellation_timeout_ms`.

Interactive sessions are stored beside the configuration file in `session.toml`. Yoctui restores the last target, screen, log filters and wrapping preference, selected backend, and up to ten recent existing build directories. Deleting this file safely resets those preferences.

## Development checks

```sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-checkout.sh
```

The strict final gate is `./scripts/verify-completion.sh`; it names any missing optional security, coverage, or profiling tool instead of reporting a false success. See `docs/testing.md` and `docs/profiling.md` for details.

## Current limitations

The bridge protocol and mocked server adapter are fully testable without Yocto. Live BitBake server control still requires validation against a supported Yocto/BitBake environment; see [docs/compatibility.md](docs/compatibility.md).
