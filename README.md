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

## Install

Use a UTF-8 Linux terminal with Git, Python 3, and stable Rust/Cargo. Your
Poky release also requires its documented host packages.

```sh
export YOCTUI_DIR="$HOME/projects/yoctui"

command -v git python3 rustc cargo
git clone https://github.com/0xbadcaffe/yoctui.git "$YOCTUI_DIR"
cd "$YOCTUI_DIR"
cargo install --locked --path crates/yoctui-cli
yoctui --help
```

For development, use:

```sh
cd "$YOCTUI_DIR"
cargo build --locked -p yoctui
cargo build --locked --release -p yoctui
```

## Quickstart: Poky build environment

Start from a complete Poky checkout containing `oe-init-build-env`. Set
`BUILDDIR` before sourcing Poky's environment script: it is both the directory
Poky creates/uses for the build and the directory Yoctui opens.

```sh
export POKY_DIR="$HOME/src/poky"
export BUILDDIR="$POKY_DIR/build-yoctui"

test -f "$POKY_DIR/oe-init-build-env" || {
  echo "missing $POKY_DIR/oe-init-build-env; use a complete Poky release" >&2
  exit 1
}
source "$POKY_DIR/oe-init-build-env" "$BUILDDIR"

yoctui --backend bridge --build-dir "$BUILDDIR"
```

Inside Yoctui, press `B`, press `e`, enter `core-image-minimal`, select the
build action, and confirm it. The first BitBake build starts from that explicit
TUI confirmation.

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
