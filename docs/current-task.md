# Current task

## Active task

**ID:** PKG-001
**Title:** Package data browser

## Objective

Implement a typed package-data workflow backed by authoritative
`oe-pkgdata-util` results, with bounded model state, shell-free acquisition,
responsive package navigation, and honest live compatibility evidence.

## Required work

1. Inventory existing package/image/recipe state, Navigator entries, UI
   specification, process adapters, output bounds/cancellation, installed
   `oe-pkgdata-util` behavior, configured pkgdata paths, and any package tests.
2. If the model, adapter, and UI work cannot remain one coherent commit, split
   `PKG-001` into atomic dependency-ordered child tasks in the registry and
   status documents, select the first child, commit that governance change,
   and continue immediately.
3. Define typed package identities and explicit not-loaded, loading,
   available-empty, available, partial, and failed states. Keep files,
   runtime dependencies/reverse dependencies, recipe/provider ownership,
   package size, and license fields explicitly unavailable until authoritative
   sources report them.
4. Implement shell-free bounded `oe-pkgdata-util` command plans and parsing.
   Validate exact arguments and configured paths, correlate responses, and
   cover missing tools/data, malformed/truncated output, nonzero exit,
   cancellation, and live read-only behavior.
5. Add a responsive Packages workspace and Navigator route with app-owned
   search, selection, refresh, dependency navigation, recipe/provider opening,
   explicit states, limitations, and contextual footer hints.
6. Add model, app, fake-process/integration, CLI, and Ratatui TestBackend tests
   named `pkgdata` (or child-task names if split).
7. Update `docs/ui-spec.md` with intentional package behavior and
   `docs/architecture.md` with proven ownership/tool boundaries.
8. Record exact live Yocto/BitBake/tool versions, commands, observed package
   coverage, and limitations without treating mocked results as live support.

## Definition of done

- Package data remains typed, bounded, correlated, and authoritative.
- Package inspection and navigation are responsive and safe at narrow sizes.
- Missing or partial data is explicit.
- Live compatibility evidence and all focused/baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model pkgdata
cargo test -p yoctui-bitbake pkgdata
cargo test -p yoctui-app pkgdata
cargo test -p yoctui-ui pkgdata
cargo test -p yoctui -- pkgdata
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible task after `PKG-001` from `docs/task-registry.toml`.
