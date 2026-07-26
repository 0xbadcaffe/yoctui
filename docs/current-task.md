# Current task

## Active task

**ID:** DEP-001
**Title:** Dependency exploration and why-built workflow

## Objective

Turn the existing dependency summary into an authoritative navigable
recipe/task dependency workspace with why-built paths and explicit partial
states.

## Required work

1. Inventory existing dependency model/events/backend methods, Recipes routing,
   workspace selection/rendering, and retained build task context.
2. Split this task in the registry first if authoritative graph acquisition,
   path derivation, and UI integration cannot remain one coherent commit.
3. Use typed backend or tool-adapter data for recipe/task edges; widgets must
   not parse raw BitBake or dot output.
4. Represent direct build/runtime dependencies, reverse dependencies, and
   why-built paths with stable typed identities and explicit unavailable,
   loading, empty, partial, and failed states.
5. Provide bounded selection/navigation from a dependency to its recipe,
   provider, task/log context, or why-built path when authoritative data exists.
6. Add fake adapter/integration, reducer/app, and responsive Ratatui
   TestBackend tests named `dependency`.
7. Update UI and architecture documents for intentional behavior and boundary
   changes.

## Definition of done

- Dependency edges and paths come from typed authoritative data.
- Direct, reverse, runtime, and why-built states are distinguishable.
- Navigation is stable, bounded, responsive, and honest about missing data.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake dependency
cargo test -p yoctui-model dependency
cargo test -p yoctui-app dependency
cargo test -p yoctui-ui dependency
cargo test -p yoctui -- dependency
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`SIG-001 — Signature dump and comparison workflows`
