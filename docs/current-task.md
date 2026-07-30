# Current task

## Active task

**ID:** SEC-CLI-001
**Title:** Integrate Security execution in the CLI

## Objective

Connect the typed Security model, adapters, and renderer to non-blocking CLI
execution while reusing the existing managed BitBake coordinator.

## Required work

1. Inspect existing CLI operation coordinators and every Security effect before
   adding state or polling.
2. Snapshot the initialized build directory and child-only environment for
   Security capability discovery without mutating process-global state.
3. Route CVE checks and recipe/image SBOM generation through the existing
   managed BitBake build coordinator with exact Security session correlation.
4. Own at most one independent report-acquisition request and one package
   mapper runner; poll bounded typed events without blocking terminal input or
   unrelated jobs.
5. Execute explicit replaceable-generation imports and exact open effects,
   mapping all adapter responses mechanically into typed reducer actions.
6. Refresh authoritative report roots after successful managed builds and
   package mapping without fabricating success when reports are empty.
7. Route cancellation only to the exact active Security operation and preserve
   rejection, timeout, nonzero, cancellation, loss, stale response, and
   success-without-reports outcomes distinctly.
8. Add fake CLI integration tests covering capability discovery, managed
   BitBake reuse, report acquisition/import/open, package mapping, refresh,
   navigation, duplicate rejection, and every terminal outcome.

## Definition of done

- Every Security effect is executed or rejected with an explicit typed action.
- Security-owned polling remains responsive and independent from other
  operation families.
- Exact request/session/report identities survive every CLI boundary.
- Managed BitBake Security operations reuse the established build coordinator.
- No fake-process test is described as live Yocto compatibility.

## Verification

```bash
cargo test -p yoctui -- security_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
