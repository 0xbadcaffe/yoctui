# Yoctui

Yoctui is a Rust/Ratatui control frontend for Yocto/BitBake. BitBake remains the metadata and build authority; Yoctui observes it and requests operations.

## Prerequisites

- Stable Rust with Cargo.
- Python 3 for the bundled bridge.
- An initialized Yocto environment when using a real BitBake workspace:

  ```sh
  source oe-init-build-env
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

Start the interactive terminal UI from an initialized build directory:

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

For scripting and CI, use a non-interactive workspace inspection:

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

Configuration is read from `$XDG_CONFIG_HOME/yoctui/config.toml` (or `~/.config/yoctui/config.toml`). CLI flags override `YOCTUI_*` environment variables, which override the configuration file, which overrides built-in defaults. Supported values include `backend`, `build_dir`, `log_retention_entries`, `log_retention_bytes`, `refresh_ms`, `default_target`, and `color`.

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
