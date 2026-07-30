# Current task

## Active task

**ID:** SEC-MODEL-001
**Title:** Model typed security workflows

## Objective

Implement pure typed Security capability, operation, report, selection, and
lifecycle state with reducer and app input/effect coverage.

## Required work

1. Add `Screen::Security` after Testing and preserve responsive navigation.
2. Model exact recipe/image scope and capability-supplied CVE, package-map,
   legacy/current recipe SBOM, and image SBOM operations.
3. Validate deterministic indexed previews without discovering tools, tasks,
   or paths in the reducer.
4. Model bounded canonical report identities, normalized CVE findings/package
   mappings, SPDX document/component summaries, limitations, and explicit
   generation-correlated inventory states.
5. Add view, scope, search/filter/drill selection, exact open effects,
   confirmation dialogs, managed-job association, cancellation, and every
   terminal action.
6. Map keys and adapter response shapes mechanically in `yoctui-app`.
7. Add focused pure-model, reducer, failure-path, and app mapping tests.

## Definition of done

- Security state is fully typed, bounded, and independent of filesystem or
  process parsing.
- Exact request, operation, report, and generation identities reject stale
  actions.
- Search, filter, drill, dialogs, lifecycle, and open effects follow the UI
  contract.
- Focused model/app and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model security_workflow
cargo test -p yoctui-app security_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
