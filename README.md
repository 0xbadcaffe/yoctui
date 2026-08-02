# Yoctui

> One terminal. Your whole Yocto workspace.

[![Rust](https://img.shields.io/badge/Rust-stable-f74c00?logo=rust)](https://www.rust-lang.org/)
[![Ratatui](https://img.shields.io/badge/UI-Ratatui-7aa2f7)](https://ratatui.rs/)
[![Yocto](https://img.shields.io/badge/Yocto-BitBake-8cc265)](https://www.yoctoproject.org/)
[![Roadmap](https://img.shields.io/badge/roadmap-155%2F155-brightgreen)](docs/implementation-status.md)

Yoctui is a Rust/Ratatui workbench for Yocto and BitBake. Browse layers and
recipes, edit metadata, run builds and Devtool, inspect dependencies, follow
tasks and logs, launch QEMU, create Wic images, and handle QA or maintenance
without losing your terminal context. BitBake remains the authority; Yoctui
organizes and controls it.

![Yoctui terminal demo](docs/media/yoctui-demo.gif)

_Recorded from the real Yoctui binary using deterministic fixture metadata so
the demo is reproducible. Live compatibility is documented separately and is
never inferred from this recording._

## What is inside

- **Build cockpit** — confirmed image/recipe builds, task progress, logs,
  structured errors, CPU/memory/disk telemetry, cancellation, and history.
- **Metadata workbench** — layer tree, recipe browser, syntax-aware preview,
  in-TUI editing, configuration provenance, BBMASK, dependencies, and
  signatures.
- **Yocto workflows** — Devtool, packages, SDK, QEMU, Wic, Testing, CVE/SPDX,
  QA, sstate, release, and maintenance tools.
- **Terminal-native UX** — responsive layouts, command palette, contextual
  shortcuts, themes, persisted sessions, shell escape, and external editor
  support.

## Prerequisites and installation

Use a UTF-8 Linux terminal with Git, Python 3, and stable Rust/Cargo. A real
Yocto build also needs the host packages required by your selected Yocto
release.

```sh
export YOCTUI_DIR="$HOME/projects/yoctui"

command -v git python3 rustc cargo
git clone https://github.com/0xbadcaffe/yoctui.git "$YOCTUI_DIR"
cd "$YOCTUI_DIR"
cargo install --locked --path crates/yoctui-cli
yoctui --help
```

For repository development, replace `cargo install` with:

```sh
cd "$YOCTUI_DIR"
cargo build --locked -p yoctui
cargo build --locked --release -p yoctui
```

## Quickstart: current Yocto development setup

Poky's current `master` migration repository may contain only a README. For
that layout, use BitBake's `bitbake-setup` command. This guarded block creates
a `qemux86-64` workspace and opens Yoctui; it does **not** start a build.

```sh
export BITBAKE_DIR="$HOME/src/bitbake"
export YOCTUI_SETUP="yoctui-qemux86-64"

test -d "$BITBAKE_DIR/.git" || \
  git clone https://git.openembedded.org/bitbake "$BITBAKE_DIR"
test -x "$BITBAKE_DIR/bin/bitbake-setup" || {
  echo "missing $BITBAKE_DIR/bin/bitbake-setup" >&2
  exit 1
}

cd "$BITBAKE_DIR"
./bin/bitbake-setup init --setup-dir-name "$YOCTUI_SETUP"

# Choose the current Poky template, poky distro, and qemux86-64 machine.
export YOCTUI_INIT="$BITBAKE_DIR/bitbake-builds/$YOCTUI_SETUP/build/init-build-env"
test -f "$YOCTUI_INIT" || {
  echo "missing $YOCTUI_INIT; review the bitbake-setup result" >&2
  exit 1
}
source "$YOCTUI_INIT"
test -n "${BUILDDIR:-}" || { echo "BUILDDIR is not set" >&2; exit 1; }

yoctui --backend bridge --build-dir "$BUILDDIR"
```

Inside Yoctui, press `B`, press `e`, enter `core-image-minimal`, select the
build action, and confirm it. The first BitBake build starts from that explicit
TUI confirmation.

## Quickstart: existing Poky checkout

Use this path for a complete Poky release checkout that contains
`oe-init-build-env`:

```sh
export YOCTO_DIR="$HOME/src/poky"
export YOCTUI_BUILD_DIR="build-yoctui"

test -f "$YOCTO_DIR/oe-init-build-env" || {
  echo "missing $YOCTO_DIR/oe-init-build-env; use a complete Poky release" >&2
  exit 1
}
source "$YOCTO_DIR/oe-init-build-env" "$YOCTUI_BUILD_DIR"
test -n "${BUILDDIR:-}" || { echo "BUILDDIR is not set" >&2; exit 1; }

yoctui --backend bridge --build-dir "$BUILDDIR"
```

## Essential controls

| Key | Action |
|---|---|
| `F5` | Build the selected target |
| `B` | Image build options |
| `r` / `y` | Recipes / Layers |
| `Ctrl+P` | Command palette |
| `Tab` | Move focus |
| `!` | Open an inherited Yocto shell; `exit` returns |
| `?` | Contextual help |
| `q` | Quit |

Shortcuts always appear in the footer. Destructive operations show an exact
preview and require confirmation.

## Performance evidence

The completion gate captured the deterministic release workload with real
`perf` samples through `cargo-flamegraph`. Open the image for the full
interactive SVG.

[![Yoctui Flamegraph](artifacts/flamegraph/yoctui.svg)](artifacts/flamegraph/yoctui.svg)

Reproduce it on a host that permits perf sampling:

```sh
cargo install flamegraph
./scripts/flamegraph.sh
```

## Learn more

- [Operator guide](docs/operator-guide.md) — daily workflows and troubleshooting
- [Compatibility evidence](docs/compatibility.md) — live, fixture, and host validation boundaries
- [UI specification](docs/ui-spec.md) — screens, focus, dialogs, and shortcuts
- [Architecture](docs/architecture.md) — crate boundaries and state flow
- [Testing](docs/testing.md) and [profiling](docs/profiling.md) — verification and performance
- [Implementation status](docs/implementation-status.md) — complete task evidence

## Development checks

```sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

Use `./scripts/verify-completion.sh` for the strict clean-checkout completion
gate. Live BitBake support is claimed only for combinations recorded in
[compatibility evidence](docs/compatibility.md).
