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

Select a backend explicitly when needed:

```sh
# Versioned Python bridge (default)
cargo run -p yoctui -- --backend bridge --build-dir "$BUILDDIR"

# Direct bitbake process fallback
cargo run -p yoctui -- --backend process --build-dir "$BUILDDIR" core-image-minimal
```

### Edit a recipe with Devtool

In the interactive Recipes screen, select a recipe and press `d`. Yoctui runs `devtool modify <recipe>` from the active build directory when needed, then opens `$BUILDDIR/workspace/sources/<recipe>` in the configured `editor` preference (or `$EDITOR`, then `vi`). Press `D` to reset that Devtool workspace; Yoctui requires confirmation before it runs `devtool reset <recipe>`. These are explicit user actions and require `devtool` from the initialized Yocto environment.

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
