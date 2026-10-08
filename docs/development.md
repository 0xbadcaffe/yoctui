# Development

Linux Rust workspace, edition 2024; CI uses Rust 1.97.0 and Python 3.12.
Install Git, a C compiler/linker, and pkg-config, then:

```bash
cargo build --locked -p yoctui -j 2
```

Run target/debug/yoctui in an 80x24+ terminal. Live BitBake requires an initialized
build; see the [user guide](user-guide.md). Use release builds for performance.

## Build from source

For development or building a specific checkout, install the
[prerequisites](https://github.com/0xbadcaffe/yoctui#install) and clone into a new directory:

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
overridden. `cargo install` uses release mode by default. Installation and build
examples limit **Cargo** to two compile jobs; they do not change BitBake/make parallelism.
Custom Cargo install roots use their own `bin` directory.

## Changing code

Read [Architecture](architecture.md), [Interface behavior](interface-behavior.md), and
[protocols](bridge-and-daemon-protocols.md). Preserve bounded state, identity, writer ownership,
terminal restoration, and exact review. Add meaningful coverage at the changed boundary.

Run [ordinary checks](testing.md#ordinary-verification) and the relevant PTY/UI/live
checks. Containers need --init or a child subreaper; do not accept zombies.
Documentation-only changes need no version bump; unchanged test reference renames
are narrowly exempt. Source/test logic/dependencies follow scripts/check-version-bump.py.
Design acceptance metadata and scene IDs remain inputs to verification.

## Optional vendor helpers

ZCU102 defaults to $HOME/src/yoctui-zcu102-2026.1; YOCTUI_ZCU102_ROOT and
YOCTUI_ZCU102_CONTAINER select an existing canonical checkout/container visible
at the same path. Helpers check identity/ownership/pins and do not create them.
OpenBMC smoke uses YOCTUI_OPENBMC_NATIVE_BUILD and YOCTUI_OPENBMC_RETAINED_BUILD;
read-only discovery does not boot a guest.

Generate temporary outputs locally; capture fresh live/performance evidence for
checks requiring it. The retained [Flamegraph](profiling.md#flamegraph) is a dated example, and the
base evidence manifest supports verifier tests. See the [documentation index](README.md).
