# Current task

## Active task

**ID:** TEST-CLI-001
**Title:** Integrate Testing execution in the CLI

## Objective

Connect the typed Testing model, adapters, and renderer to non-blocking CLI
execution while reusing the existing managed BitBake coordinator.

## Required work

1. Inspect existing CLI operation coordinators and Testing effect routing
   before adding state or polling.
2. Snapshot the initialized build directory and PATH for independent
   selftest/resulttool capability inspection without mutating process-global
   environment.
3. Route image runtime, SDK, extensible SDK, and configured ptest launches
   through the existing managed BitBake build coordinator with exact session
   correlation.
4. Own at most one independent selftest runner and one resulttool
   comparison/export runner; poll bounded typed events without blocking
   terminal input or unrelated jobs.
5. Execute explicit replaceable-generation result imports and exact
   destination inspections, map responses mechanically, and trigger retained
   exact-result refresh after successful Testing operations.
6. Route cancellation only to the exact active Testing operation and preserve
   rejection, timeout, nonzero, cancellation, loss, stale response, and
   success-without-results outcomes distinctly.
7. Add fake CLI integration tests covering capability discovery, managed
   BitBake reuse, selftest/result operations, refresh/import/export,
   navigation, duplicate rejection, and every terminal outcome.

## Definition of done

- Every Testing effect is executed or rejected with an explicit typed action.
- Testing-owned polling remains responsive and independent from other
  operation families.
- Exact request/session/result identities survive every CLI boundary.
- Managed BitBake tests reuse the established build coordinator.
- No fake-process test is described as live Yocto compatibility.

## Verification

```bash
cargo test -p yoctui -- test_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
