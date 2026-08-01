# Current Task

## Task

**ID:** MAINT-ADAPTER-001
**Title:** Close Maintenance adapter gate

## Objective

Verify the combined capability, identity, command, evidence, and lifecycle
boundary across every specified Maintenance adapter family without expanding
fixture results into unsupported live-compatibility claims.

## Required work

1. Inspect the sstate, service, release, and optional adapter implementations
   and their focused tests for duplicated or missing specified behavior.
2. Verify each family returns typed bounded capability, diagnostic, preview,
   evidence, or error values and keeps raw filesystem/process classification
   inside `yoctui-bitbake`.
3. Verify every executable and filesystem identity is canonical and revalidated
   at the relevant execution or evidence boundary; unsafe, changed, missing,
   and unsupported inputs remain explicit.
4. Verify exact shell-free commands reuse the shared Maintenance runner with
   bounded streams and distinct success, nonzero, timeout, cancellation,
   rejection, and runner-loss outcomes.
5. Verify destructive and network effects remain separately represented:
   cleanup candidates, PR data, locked-cache replacement, archive creation,
   and archive push must not collapse into one implicit operation.
6. Verify optional integrations remain detection-only and do not construct or
   run mail, upload, manifest mutation, network, or service-lifecycle actions.
7. Add only missing aggregate regression coverage found by this audit; do not
   duplicate already adequate focused tests or weaken any gate.
8. Do not claim live compatibility from fixture tests.

## Definition of done

- All four adapter families pass their focused checks together with the model
  workflow checks.
- No architecture or specification boundary is contradicted.
- The complete baseline passes.

## Verification

```bash
cargo test -p yoctui-bitbake maintenance_
cargo test -p yoctui-app maintenance_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Reconcile `docs/architecture.md` only if the audit changes a boundary.
- Mark `MAINT-ADAPTER-001` `DONE` only after verification passes.
- Update `docs/implementation-status.md`.
- Replace this file with the next eligible highest-priority Maintenance task.

## Next task

`MAINT-RENDER-001`
