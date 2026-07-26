# Current task

## Active task

**ID:** DEP-MODEL-001
**Title:** Add typed dependency graph and why-built paths

## Objective

Define pure typed recipe/task graph state with deterministic reverse-edge and
why-built path derivation, explicit partial states, and stable reducer
selection independent of any raw backend format.

## Required work

1. Inventory the current flat `RecipeDependencies`, dependency reducer actions,
   selection/navigation, and backend event normalization.
2. Add stable typed recipe/task node identities and edge kinds for build,
   runtime, and task dependencies; normalize duplicate/self/unknown edges
   deterministically.
3. Represent not loaded, loading, available-empty, partial, and failed graph
   states without fabricated values.
4. Derive reverse edges and one deterministic bounded shortest why-built path
   from typed edges only, including cycles and unreachable nodes.
5. Preserve selected identity across refresh where possible and clamp safely
   when nodes disappear.
6. Add typed reducer actions for request, success, partial success, and failure;
   keep existing direct dependency compatibility until adapter/UI migration.
7. Add model and app tests named `dependency_graph` for normalization,
   reverse edges, shortest paths, cycles, bounds, partial/failure states,
   selection stability, and typed event mapping.
8. Update architecture documentation for the graph ownership boundary.

## Definition of done

- Pure model state owns normalized graph identities, edges, reverse lookup, and
  bounded why-built paths.
- Every load and partial state is explicit.
- Selection is identity-stable and narrow-safe.
- No model or UI code parses raw BitBake/dot text.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model dependency_graph
cargo test -p yoctui-app dependency_graph
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEP-ADAPTER-001 — Acquire authoritative dependency graphs`
