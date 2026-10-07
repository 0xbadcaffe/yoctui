# Development

Yoctui is a Linux Rust workspace using edition 2024. CI pins Rust 1.97.0 with
rustfmt and Clippy and Python 3.12. Build from the existing checkout with Git,
a C compiler/linker, and pkg-config installed:

```bash
cargo build --locked -p yoctui -j 2
```

The binary is `target/debug/yoctui`. Use a real terminal of at least 80×24 for
interactive testing. An initialized Yocto workspace is needed only for live
BitBake operations; see [operator guide](operator-guide.md).

## Design and boundaries

- [Architecture](architecture.md): crate dependencies, authority, IPC and lifecycle.
- [UI specification](ui-spec.md): behavior, focus, layouts, shortcuts and safety.
- [Workbench design](workbench-design.md): interaction and widget decisions.
- [Protocol](protocol.md): wire types and compatibility.
- [Development priorities](product-roadmap.md): unresolved integration and validation.

Update the relevant specification when intentionally changing behavior. Add
meaningful reducer, application, renderer or integration coverage for the changed
boundary. Preserve bounded state, terminal restoration, writer leases, exact
workspace identity, and preview/confirmation for destructive actions.

## Verification

```bash
cargo fmt --all --check
cargo test --locked --workspace --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-design-contracts.sh
./scripts/test-readme-quickstart.sh
```

[Testing](testing.md) documents PTY, fixture, live, performance, and completion
gates. CI's container uses `--init` to reap orphaned fixture children. A container
without a child reaper can fail lifecycle tests because exited children remain
visible as zombies; use an init/subreaper rather than weakening assertions.

`docs/design/acceptance-contracts.toml` contains only metadata consumed by
existing design/evidence checks and unresolved acceptance boundaries. It is not
an agent task registry. Preserve the stable IDs referenced by scenario manifests;
current behavior is verified by tests and evidence, not completion labels alone.

No product version bump is needed for documentation-only changes. Rust source,
Rust tests and dependency changes follow `scripts/check-version-bump.py`.

## Optional vendor validation helpers

ZCU102 helpers default to `$HOME/src/yoctui-zcu102-2026.1`.
`YOCTUI_ZCU102_ROOT` selects another existing canonical absolute checkout;
`YOCTUI_ZCU102_CONTAINER` selects its validation container. Host and container
must see the checkout at the same path. The inspector checks peer ownership and
exact daemon/build authority. Cache preseed checks root ownership, pinned source
identity and BitBake's fetch lock and never replaces an existing cache. These
helpers do not create the checkout/container or override source pins.

The ignored OpenBMC defaults smoke requires `YOCTUI_OPENBMC_NATIVE_BUILD` for
the native build without matching symbols and `YOCTUI_OPENBMC_RETAINED_BUILD`
for the retained build with matching symbols. Read-only discovery does not launch
a guest. Values must identify the user's actual initialized workspaces.

## Retained README assets and external evidence

The repository retains only the flamegraph, its summary, and source captures
and manifest needed by README media. Historical performance, board-debugging
and release-validation outputs are no longer bundled. Capture fresh evidence
before running checks that read `artifacts/performance/` or complete live UI
bundles. Missing evidence is a prerequisite failure, not a passing check or
proof that a measurement remains valid for the current release. The evidence
verifiers and their assertions remain intact.
