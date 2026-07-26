# Current task

## Active task

**ID:** DEP-ADAPTER-001
**Title:** Acquire authoritative dependency graphs

## Objective

Acquire bounded recipe and task dependency data from authoritative BitBake
interfaces, normalize all raw formats inside adapter boundaries, and emit the
typed graph states already owned by the model.

## Required work

1. Inventory the current bridge `get_dependencies` request, Tinfoil datastore
   fields, process-backend capabilities, protocol compatibility behavior, and
   available `bitbake -g`/`oe-depends-dot` outputs.
2. Define a backward-compatible typed graph protocol payload for recipe/task
   nodes and build/runtime/task edges, including explicit limitations and
   bounded counts.
3. Prefer BitBake server/Tinfoil APIs where authoritative data is available;
   use a shell-free bounded tool adapter only for graph information the server
   cannot supply.
4. Keep dot/text parsing exclusively in `yoctui-bitbake` or the Python bridge;
   reject malformed, oversized, ambiguous, and path-escaping records without
   leaking raw text into the app, model, or UI.
5. Correlate every success, partial result, and failure with the requested
   typed root identity. Do not fabricate task or runtime edges when the active
   backend cannot report them.
6. Preserve the legacy flat dependency response for compatibility while
   routing capable backends through typed `DependencyGraph` events.
7. Add fake-process, protocol round-trip/backward-compatibility, bridge, and
   app normalization tests named `dependency_graph`; cover empty, partial,
   malformed, bounded, nonzero-exit, and unavailable-tool cases.
8. Add live-Yocto smoke coverage that records the exact BitBake release and
   command/API exercised. If the external environment cannot supply the
   required interface, mark only that validation BLOCKED with exact
   reproduction details; do not claim live support from mocks.
9. Update architecture documentation for the selected acquisition boundary.

## Definition of done

- Capable backends emit normalized typed graphs with honest limitations.
- Raw BitBake, dot, and process output never crosses the adapter boundary.
- Resource limits and all failure modes are explicit and tested.
- Legacy peers remain compatible.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake dependency_graph
cargo test -p yoctui-protocol dependency_graph
cargo test -p yoctui-app dependency_graph
python3 -m pytest bridge/tests -k dependenc
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEP-UI-001 — Integrate dependency and why-built workspace`
